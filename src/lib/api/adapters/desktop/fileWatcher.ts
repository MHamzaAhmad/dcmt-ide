// Desktop File Watcher (Tauri Events)
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { eventStore } from '$lib/stores/events';

export interface FileEvent {
	event_type: 'Created' | 'Modified' | 'Deleted' | 'Renamed';
	path: string;
	metadata: {
		is_directory: boolean;
		size?: number;
		old_path?: string;
		new_path?: string;
	};
	timestamp: number;
}

export type FileEventCallback = (event: FileEvent) => void;

export class DesktopFileWatcher {
	private listeners: UnlistenFn[] = [];
	private eventCallbacks: Map<string, FileEventCallback[]> = new Map();

	constructor() {
		this.initializeEventListeners();
	}

	private async initializeEventListeners() {
		try {
			// Listen to file-created events
			const createdUnlisten = await listen<FileEvent>('file-created', (event) => {
				this.handleFileEvent('created', event.payload);
			});
			this.listeners.push(createdUnlisten);

			// Listen to file-modified events
			const modifiedUnlisten = await listen<FileEvent>('file-modified', (event) => {
				this.handleFileEvent('modified', event.payload);
			});
			this.listeners.push(modifiedUnlisten);

			// Listen to file-deleted events
			const deletedUnlisten = await listen<FileEvent>('file-deleted', (event) => {
				this.handleFileEvent('deleted', event.payload);
			});
			this.listeners.push(deletedUnlisten);

			// Listen to file-renamed events
			const renamedUnlisten = await listen<FileEvent>('file-renamed', (event) => {
				this.handleFileEvent('renamed', event.payload);
			});
			this.listeners.push(renamedUnlisten);

			console.log('Desktop file watcher initialized - listening to Tauri events');
		} catch (error) {
			console.error('Failed to initialize desktop file watcher:', error);
		}
	}

	private handleFileEvent(type: string, event: FileEvent) {
		console.log(`File ${type}:`, event);
		
		// Emit to EventStore first
		const source = 'watcher';
		switch (type.toLowerCase()) {
			case 'created':
				eventStore.events.fileCreated(event.path, event.metadata.is_directory, source);
				break;
			case 'modified':
				eventStore.events.fileModified(event.path, event.metadata.size, source);
				break;
			case 'deleted':
				eventStore.events.fileDeleted(event.path, event.metadata.is_directory, source);
				break;
			case 'renamed':
				if (event.metadata.old_path) {
					eventStore.events.fileRenamed(event.path, event.metadata.old_path, event.metadata.is_directory, source);
				} else {
					eventStore.events.fileModified(event.path, event.metadata.size, source);
				}
				break;
		}
		
		// Call registered callbacks
		const callbacks = this.eventCallbacks.get(type) || [];
		callbacks.forEach(callback => {
			try {
				callback(event);
			} catch (error) {
				console.error(`Error in file event callback for ${type}:`, error);
			}
		});

		// Note: Browser events removed - all events now go through EventStore for consistency
	}

	/**
	 * Subscribe to file events
	 * @param eventType - Type of event to listen for
	 * @param callback - Callback function to call when event occurs
	 * @returns Unsubscribe function
	 */
	on(eventType: 'created' | 'modified' | 'deleted' | 'renamed', callback: FileEventCallback): () => void {
		if (!this.eventCallbacks.has(eventType)) {
			this.eventCallbacks.set(eventType, []);
		}
		
		const callbacks = this.eventCallbacks.get(eventType)!;
		callbacks.push(callback);

		// Return unsubscribe function
		return () => {
			const index = callbacks.indexOf(callback);
			if (index > -1) {
				callbacks.splice(index, 1);
			}
		};
	}

	/**
	 * Subscribe to all file events
	 * @param callback - Callback function to call for any file event
	 * @returns Unsubscribe function
	 */
	onAny(callback: (type: string, event: FileEvent) => void): () => void {
		const unsubscribers = [
			this.on('created', (event) => callback('created', event)),
			this.on('modified', (event) => callback('modified', event)),
			this.on('deleted', (event) => callback('deleted', event)),
			this.on('renamed', (event) => callback('renamed', event))
		];

		return () => {
			unsubscribers.forEach(unsub => unsub());
		};
	}

	/**
	 * Check if file watcher is active
	 */
	isActive(): boolean {
		return this.listeners.length > 0;
	}

	/**
	 * Clean up event listeners
	 */
	async destroy(): Promise<void> {
		try {
			// Unlisten from all Tauri events
			await Promise.all(this.listeners.map(unlisten => unlisten()));
			this.listeners = [];
			
			// Clear callbacks
			this.eventCallbacks.clear();
			
			console.log('Desktop file watcher destroyed');
		} catch (error) {
			console.error('Error destroying desktop file watcher:', error);
		}
	}
}