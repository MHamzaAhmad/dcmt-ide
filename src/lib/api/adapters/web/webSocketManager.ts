// WebSocketManager - Singleton WebSocket connection manager
// Ensures only one WebSocket connection exists per browser tab

import type { FileEvent } from './fileWebSocket';
import type { CompilationEvent } from '../../types';

export interface FileEventCallback {
    (type: string, event: FileEvent): void;
}

export interface CompilationEventCallback {
    (event: CompilationEvent): void;
}

export class WebSocketManager {
    private static instance: WebSocketManager | null = null;
    private ws: WebSocket | null = null;
    private isConnected: boolean = false;
    private reconnectAttempts: number = 0;
    private maxReconnectAttempts: number = 10;
    private baseReconnectDelay: number = 1000; // 1 second
    private reconnectTimeout: number | null = null;

    // Event subscribers
    private fileEventCallbacks: FileEventCallback[] = [];
    private compilationEventCallbacks: CompilationEventCallback[] = [];
    private connectionCallbacks: ((connected: boolean) => void)[] = [];

    // Connection stats
    private connectionCount: number = 0;
    private lastConnectedAt: number | null = null;
    private lastDisconnectedAt: number | null = null;
    private disconnectReason: string | null = null;

    private constructor() {
        console.log('WebSocketManager: Initializing singleton instance');
    }

    static getInstance(): WebSocketManager {
        if (!WebSocketManager.instance) {
            WebSocketManager.instance = new WebSocketManager();
        }
        return WebSocketManager.instance;
    }

    // Connect to WebSocket if not already connected
    connect(): void {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            console.log('WebSocketManager: Already connected');
            return;
        }

        console.log('WebSocketManager: Creating new WebSocket connection...');

        try {
            const protocol = import.meta.env.VITE_API_BASE_URL?.startsWith('https') ? 'wss:' : 'ws:';
            const baseUrl = import.meta.env.VITE_API_BASE_URL?.replace(/^(https?:\/\/)/, '') || 'localhost:3001';
            const wsUrl = `${protocol}//${baseUrl}/ws`;

            this.ws = new WebSocket(wsUrl);
            this.connectionCount++;

            this.ws.onopen = () => {
                console.log(`WebSocketManager: Connected (attempt #${this.reconnectAttempts + 1})`);
                this.isConnected = true;
                this.reconnectAttempts = 0;
                this.lastConnectedAt = Date.now();
                this.disconnectReason = null;

                // Auto-subscribe to events
                this.handleConnectionOpen();

                // Notify connection callbacks
                this.connectionCallbacks.forEach(callback => callback(true));
            };

            this.ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    console.log('WebSocketManager: Received event:', data);

                    // Handle compilation events
                    if (data.type === 'compilation_event') {
                        this.handleCompilationEvent(data.event);
                    }

                    // Handle file events
                    if (data.type === 'file_event' && data.event) {
                        this.handleFileEvent(data.event.event_type, data.event);
                    }

                    // Handle connection status
                    if (data.type === 'connection') {
                        console.log('WebSocketManager: Connection status:', data.status);
                    }
                } catch (error) {
                    console.error('WebSocketManager: Failed to parse WebSocket message:', error);
                }
            };

            this.ws.onerror = (error) => {
                console.error('WebSocketManager: WebSocket error:', error);
            };

            this.ws.onclose = (event) => {
                console.log(`WebSocketManager: Disconnected - Code: ${event.code}, Reason: ${event.reason}`);
                this.isConnected = false;
                this.lastDisconnectedAt = Date.now();
                this.disconnectReason = `${event.code}: ${event.reason}`;
                this.ws = null;

                // Notify connection callbacks
                this.connectionCallbacks.forEach(callback => callback(false));

                // Attempt to reconnect
                this.scheduleReconnect();
            };

        } catch (error) {
            console.error('WebSocketManager: Failed to create WebSocket:', error);
            this.scheduleReconnect();
        }
    }

    // Disconnect WebSocket
    disconnect(): void {
        if (this.reconnectTimeout) {
            clearTimeout(this.reconnectTimeout);
            this.reconnectTimeout = null;
        }

        if (this.ws) {
            console.log('WebSocketManager: Disconnecting...');
            this.ws.close(1000, 'Manual disconnect');
            this.ws = null;
        }

        this.isConnected = false;
        this.lastDisconnectedAt = Date.now();
        this.disconnectReason = 'Manual disconnect';

        // Notify connection callbacks
        this.connectionCallbacks.forEach(callback => callback(false));
    }

    // Send message to server
    send(data: any): void {
        if (this.isConnected && this.ws) {
            this.ws.send(JSON.stringify(data));
        } else {
            console.warn('WebSocketManager: Cannot send message - not connected');
        }
    }

    // Subscribe to file events
    onFileEvent(callback: FileEventCallback): () => void {
        this.fileEventCallbacks.push(callback);
        console.log(`WebSocketManager: Added file event callback, total: ${this.fileEventCallbacks.length}`);

        // Auto-subscribe if connected
        if (this.isConnected) {
            this.send({ type: 'subscribe_files' });
        }

        // Return unsubscribe function
        return () => {
            const index = this.fileEventCallbacks.indexOf(callback);
            if (index > -1) {
                this.fileEventCallbacks.splice(index, 1);
                console.log(`WebSocketManager: Removed file event callback, remaining: ${this.fileEventCallbacks.length}`);
            }
        };
    }

    // Subscribe to compilation events
    onCompilationEvent(callback: CompilationEventCallback): () => void {
        this.compilationEventCallbacks.push(callback);
        console.log(`WebSocketManager: Added compilation event callback, total: ${this.compilationEventCallbacks.length}`);

        // Auto-subscribe if connected
        if (this.isConnected) {
            this.send({ type: 'subscribe_compilation' });
        }

        // Return unsubscribe function
        return () => {
            const index = this.compilationEventCallbacks.indexOf(callback);
            if (index > -1) {
                this.compilationEventCallbacks.splice(index, 1);
                console.log(`WebSocketManager: Removed compilation event callback, remaining: ${this.compilationEventCallbacks.length}`);
            }
        };
    }

    // Subscribe to connection status changes
    onConnectionChange(callback: (connected: boolean) => void): () => void {
        this.connectionCallbacks.push(callback);

        // Return unsubscribe function
        return () => {
            const index = this.connectionCallbacks.indexOf(callback);
            if (index > -1) {
                this.connectionCallbacks.splice(index, 1);
            }
        };
    }

    // Get connection stats
    getStats() {
        return {
            isConnected: this.isConnected,
            connectionCount: this.connectionCount,
            reconnectAttempts: this.reconnectAttempts,
            lastConnectedAt: this.lastConnectedAt,
            lastDisconnectedAt: this.lastDisconnectedAt,
            disconnectReason: this.disconnectReason,
            fileEventCallbacks: this.fileEventCallbacks.length,
            compilationEventCallbacks: this.compilationEventCallbacks.length
        };
    }

    private handleConnectionOpen(): void {
        // Subscribe to file events if we have callbacks
        if (this.fileEventCallbacks.length > 0) {
            this.send({ type: 'subscribe_files' });
            console.log('WebSocketManager: Auto-subscribed to file events');
        }

        // Subscribe to compilation events if we have callbacks
        if (this.compilationEventCallbacks.length > 0) {
            this.send({ type: 'subscribe_compilation' });
            console.log('WebSocketManager: Auto-subscribed to compilation events');
        }
    }

    private handleFileEvent(type: string, event: FileEvent): void {
        console.log(`WebSocketManager: Handling file event ${type} for ${event.path}`);
        this.fileEventCallbacks.forEach((callback, index) => {
            try {
                callback(type, event);
            } catch (error) {
                console.error(`WebSocketManager: Error in file event callback ${index + 1}:`, error);
            }
        });
    }

    private handleCompilationEvent(event: CompilationEvent): void {
        console.log(`WebSocketManager: Handling compilation event: ${event.event_type}`);
        this.compilationEventCallbacks.forEach((callback, index) => {
            try {
                callback(event);
            } catch (error) {
                console.error(`WebSocketManager: Error in compilation event callback ${index + 1}:`, error);
            }
        });
    }

    private scheduleReconnect(): void {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.error('WebSocketManager: Max reconnection attempts reached');
            return;
        }

        const delay = Math.min(
            this.baseReconnectDelay * Math.pow(2, this.reconnectAttempts),
            30000 // Max 30 seconds
        );

        console.log(`WebSocketManager: Scheduling reconnection in ${delay}ms (attempt ${this.reconnectAttempts + 1}/${this.maxReconnectAttempts})`);

        this.reconnectTimeout = window.setTimeout(() => {
            this.reconnectAttempts++;
            this.connect();
        }, delay);
    }
}

// Export singleton instance
export const webSocketManager = WebSocketManager.getInstance();