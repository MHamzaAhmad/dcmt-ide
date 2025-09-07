// Web File WebSocket Adapter for File Watching
import { apiClient } from '../../client';
import type { FileWatcherOperations } from '../../types';
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

export class WebFileWatcher {
	private eventCallbacks: Map<string, FileEventCallback[]> = new Map();
	private websocket: WebSocketAdapter;

	constructor() {
		this.websocket = new WebSocketAdapter();
		this.initializeWebSocketListeners();
	}

	private initializeWebSocketListeners() {
		// Listen to browser events emitted by WebSocket adapter
		const eventHandler = (e: CustomEvent) => {
			const { type, event } = e.detail;
			if (event && typeof type === 'string') {
				this.handleFileEvent(type, event);
			}
		};

		window.addEventListener('file-event', eventHandler as EventListener);
		console.log('WebFileWatcher initialized - listening to WebSocket events');
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
		return this.websocket.isConnected();
	}

	/**
	 * Clean up event listeners
	 */
	async destroy(): Promise<void> {
		try {
			// Clear callbacks
			this.eventCallbacks.clear();
			
			// Destroy WebSocket connection
			await this.websocket.destroy();
			
			console.log('WebFileWatcher destroyed');
		} catch (error) {
			console.error('Error destroying WebFileWatcher:', error);
		}
	}

	/**
	 * Get underlying WebSocket adapter
	 */
	getWebSocket(): WebSocketAdapter {
		return this.websocket;
	}
}

export class WebSocketAdapter implements FileWatcherOperations {
	private ws: WebSocket | null = null;

	// WebSocket support for file watching (web mode only)
	connect(): WebSocket | null {
		if (this.ws && this.ws.readyState === WebSocket.OPEN) {
			return this.ws;
		}

		try {
			const wsUrl = apiClient.baseURL.replace(/^http/, 'ws') + '/ws';
			this.ws = new WebSocket(wsUrl);
			
			this.ws.onopen = () => {
				console.log('WebSocket connected for file watching');
			};
			
			this.ws.onmessage = (event) => {
				try {
					const data = JSON.parse(event.data);
					console.log('WebSocket file event:', data);
					// Emit custom events that components can listen to
					if (typeof window !== 'undefined') {
						window.dispatchEvent(new CustomEvent('file-event', { detail: data }));
					}
				} catch (error) {
					console.error('Failed to parse WebSocket message:', error);
				}
			};
			
			this.ws.onerror = (error) => {
				console.error('WebSocket error:', error);
			};
			
			this.ws.onclose = (event) => {
				console.log('WebSocket disconnected:', event.code, event.reason);
				this.ws = null;
				
				// Attempt to reconnect after a delay
				setTimeout(() => {
					if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
						console.log('Attempting to reconnect WebSocket...');
						this.connect();
					}
				}, 5000);
			};
			
			return this.ws;
		} catch (error) {
			console.error('Failed to connect WebSocket:', error);
			return null;
		}
	}

	disconnect(): void {
		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}
	}

	// Implement FileWatcherOperations interface
	async destroy(): Promise<void> {
		this.disconnect();
	}

	isActive(): boolean {
		return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
	}

	isConnected(): boolean {
		return this.isActive();
	}

	// Send message to server if needed
	send(data: any): void {
		if (this.isConnected() && this.ws) {
			this.ws.send(JSON.stringify(data));
		}
	}
}