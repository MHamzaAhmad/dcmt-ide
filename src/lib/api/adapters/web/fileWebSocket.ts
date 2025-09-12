// Web File WebSocket Adapter for File Watching
import type { FileWatcherOperations, CompilationEvent } from '../../types';
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
	private isPaused: boolean = false;

	constructor() {
		this.websocket = new WebSocketAdapter();
		this.initializeWebSocketListeners();
	}

	private initializeWebSocketListeners() {
		console.log('WebFileWatcher: Initializing WebSocket listeners');
		
		// Subscribe to file events directly via WebSocket
		this.websocket.onFileEvents((type, event) => {
			console.log('WebFileWatcher: Received event from WebSocket:', type, event);
			this.handleFileEvent(type, event);
		});
		
		// Connect WebSocket if not already connected
		if (!this.websocket.isConnected()) {
			console.log('WebFileWatcher: WebSocket not connected, initiating connection...');
			this.websocket.connect();
		} else {
			console.log('WebFileWatcher: WebSocket already connected');
		}
		
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
	 * Pause file event processing (events will be ignored)
	 */
	pause(): void {
		this.isPaused = true;
		console.log('[WebFileWatcher] File event processing paused');
	}

	/**
	 * Resume file event processing
	 */
	resume(): void {
		this.isPaused = false;
		console.log('[WebFileWatcher] File event processing resumed');
	}

	/**
	 * Check if file watcher is paused
	 */
	isPausedState(): boolean {
		return this.isPaused;
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
	private compilationEventCallbacks: ((event: CompilationEvent) => void)[] = [];
	private fileEventCallbacks: ((type: string, event: FileEvent) => void)[] = [];

	// WebSocket support for file watching (web mode only)
	connect(): WebSocket | null {
		if (this.ws && this.ws.readyState === WebSocket.OPEN) {
			return this.ws;
		}

		try {
			// Use relative URL to let nginx handle proxying in Docker deployment
			// Construct WebSocket URL based on current protocol and host
			const protocol = import.meta.env.VITE_API_BASE_URL.startsWith('https') ? 'wss:' : 'ws:';
            const baseUrl = import.meta.env.VITE_API_BASE_URL.replace(/^(https?:\/\/)/, '');
			const wsUrl = `${protocol}//${baseUrl}/ws`;
			this.ws = new WebSocket(wsUrl);
			
			this.ws.onopen = () => {
				console.log('WebSocket connected for file watching');
				
				// Auto-subscribe to file events if we have callbacks
				if (this.fileEventCallbacks.length > 0) {
					this.send({ type: 'subscribe_files' });
					console.log('WebSocket: Auto-subscribed to file events on connection');
				}
				
				// Auto-subscribe to compilation events if we have callbacks
				if (this.compilationEventCallbacks.length > 0) {
					this.send({ type: 'subscribe_compilation' });
					console.log('WebSocket: Auto-subscribed to compilation events on connection');
				}
			};
			
			this.ws.onmessage = (event) => {
				try {
					const data = JSON.parse(event.data);
					console.log('WebSocket event:', data);
					
					// Handle compilation events
					if (data.type === 'compilation_event') {
						this.handleCompilationEvent(data.event);
					}
					
					// Handle file events
					if (data.type === 'file_event' && data.event) {
						this.handleFileEvent(data.event.event_type, data.event);
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

	// File event handling
	private handleFileEvent(type: string, event: FileEvent): void {
		console.log('WebSocket: Received file event:', type, event);
		console.log('WebSocket: File event callbacks count:', this.fileEventCallbacks.length);
		this.fileEventCallbacks.forEach((callback, index) => {
			try {
				console.log(`WebSocket: Calling file event callback ${index + 1}/${this.fileEventCallbacks.length}`);
				callback(type, event);
			} catch (error) {
				console.error('Error in file event callback:', error);
			}
		});
	}

	// Compilation event handling
	private handleCompilationEvent(event: CompilationEvent): void {
		console.log('Received compilation event:', event);
		this.compilationEventCallbacks.forEach(callback => {
			try {
				callback(event);
			} catch (error) {
				console.error('Error in compilation event callback:', error);
			}
		});
	}

	onFileEvents(callback: (type: string, event: FileEvent) => void): () => void {
		this.fileEventCallbacks.push(callback);
		
		console.log('WebSocket: Added file event callback, total callbacks:', this.fileEventCallbacks.length);
		
		// Subscribe to file events if connected, or it will auto-subscribe on connection
		if (this.isConnected()) {
			this.send({ type: 'subscribe_files' });
			console.log('WebSocket: Subscribed to file events (already connected)');
		} else {
			console.log('WebSocket: Will subscribe to file events when connected');
		}
		
		return () => {
			const index = this.fileEventCallbacks.indexOf(callback);
			if (index > -1) {
				this.fileEventCallbacks.splice(index, 1);
			}
			
			console.log('WebSocket: Removed file event callback, remaining callbacks:', this.fileEventCallbacks.length);
			
			// Unsubscribe if no more callbacks
			if (this.fileEventCallbacks.length === 0 && this.isConnected()) {
				this.send({ type: 'unsubscribe_files' });
				console.log('WebSocket: Unsubscribed from file events (no more callbacks)');
			}
		};
	}

	onCompilationEvent(callback: (event: CompilationEvent) => void): () => void {
		this.compilationEventCallbacks.push(callback);
		
		// Subscribe to compilation events on connect
		if (this.isConnected()) {
			this.send({ type: 'subscribe_compilation' });
		}
		
		return () => {
			const index = this.compilationEventCallbacks.indexOf(callback);
			if (index > -1) {
				this.compilationEventCallbacks.splice(index, 1);
			}
			
			// Unsubscribe if no more callbacks
			if (this.compilationEventCallbacks.length === 0 && this.isConnected()) {
				this.send({ type: 'unsubscribe_compilation' });
			}
		};
	}
}