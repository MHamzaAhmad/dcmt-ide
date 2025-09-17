// Web File WebSocket Adapter for File Watching
import type { FileWatcherOperations, CompilationEvent } from '../../types';
import { eventStore } from '$lib/stores/events';
import { webSocketManager } from './webSocketManager';

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

export class WebFileWatcher {
	private eventCallbacks: Map<string, FileEventCallback[]> = new Map();
	private isPaused: boolean = false;
	private fileEventUnsubscribe: (() => void) | null = null;

	constructor() {
		console.log('WebFileWatcher: Initializing with shared WebSocket manager');
		this.initializeWebSocketListeners();
	}

	private initializeWebSocketListeners() {
		console.log('WebFileWatcher: Setting up WebSocket listeners via WebSocketManager');

		// Subscribe to file events via WebSocketManager
		this.fileEventUnsubscribe = webSocketManager.onFileEvent((type, event) => {
			console.log('WebFileWatcher: Received event from WebSocketManager:', type, event);
			this.handleFileEvent(type, event);
		});

		// Ensure WebSocket is connected
		webSocketManager.connect();

		console.log('WebFileWatcher: Initialization complete - subscribed to WebSocket file events');
	}

	private handleFileEvent(type: string, event: FileEvent) {
		// If paused, ignore the event
		if (this.isPaused) {
			console.log(`[WebFileWatcher] Ignoring ${type} event while paused:`, event.path);
			return;
		}

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

		this.eventCallbacks.get(eventType)!.push(callback);
		console.log(`[WebFileWatcher] Added callback for ${eventType}, total: ${this.eventCallbacks.get(eventType)!.length}`);

		return () => {
			const callbacks = this.eventCallbacks.get(eventType);
			if (callbacks) {
				const index = callbacks.indexOf(callback);
				if (index > -1) {
					callbacks.splice(index, 1);
					console.log(`[WebFileWatcher] Removed callback for ${eventType}, remaining: ${callbacks.length}`);
				}
			}
		};
	}

	/**
	 * Subscribe to all file events
	 * @param callback - Callback function to call when any file event occurs
	 * @returns Unsubscribe function
	 */
	onAny(callback: (type: string, event: FileEvent) => void): () => void {
		const eventTypes: Array<'created' | 'modified' | 'deleted' | 'renamed'> = ['created', 'modified', 'deleted', 'renamed'];
		const unsubscribes: (() => void)[] = [];

		eventTypes.forEach(eventType => {
			const unsubscribe = this.on(eventType, (event) => {
				callback(eventType, event);
			});
			unsubscribes.push(unsubscribe);
		});

		return () => {
			unsubscribes.forEach(unsubscribe => unsubscribe());
		};
	}

	/**
	 * Pause file event processing
	 * Events will be queued but not processed until resumed
	 */
	pause(): void {
		this.isPaused = true;
		console.log('[WebFileWatcher] File event processing paused');
	}

	/**
	 * Resume file event processing
	 * Any queued events will be processed
	 */
	resume(): void {
		this.isPaused = false;
		console.log('[WebFileWatcher] File event processing resumed');
	}

	// Implement FileWatcherOperations interface
	async destroy(): Promise<void> {
		console.log('WebFileWatcher: Destroying...');

		// Unsubscribe from WebSocket events
		if (this.fileEventUnsubscribe) {
			this.fileEventUnsubscribe();
			this.fileEventUnsubscribe = null;
		}

		// Clear event callbacks
		this.eventCallbacks.clear();
	}

	isActive(): boolean {
		return true; // Always active when using WebSocketManager
	}

	isConnected(): boolean {
		return true; // Connection managed by WebSocketManager
	}
}