// WebSocketManager - Singleton WebSocket connection manager
// Ensures only one WebSocket connection exists per browser tab

import type { FileEvent } from './fileWebSocket';
import type { CompilationEvent } from '../../types';
import { getWebSocketUrl } from '$lib/utils/api';

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
    // Small buffer to hold recent compilation events until a subscriber attaches
    private compilationEventBuffer: CompilationEvent[] = [];
    private maxCompilationBuffer: number = 50;

    // Message queue for messages sent while connecting
    private messageQueue: any[] = [];

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

    // Connect to WebSocket if not already connected or connecting
    connect(): void {
        if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
            console.log(`WebSocketManager: Already ${this.ws.readyState === WebSocket.OPEN ? 'connected' : 'connecting'}`);
            return;
        }

        console.log('WebSocketManager: Creating new WebSocket connection...');

        try {
            const wsUrl = `${getWebSocketUrl()}/ws`;

            this.ws = new WebSocket(wsUrl);
            this.connectionCount++;

            this.ws.onopen = () => {
                console.log(`WebSocketManager: Connected (attempt #${this.reconnectAttempts + 1})`);
                this.isConnected = true;
                this.reconnectAttempts = 0;
                this.lastConnectedAt = Date.now();
                this.disconnectReason = null;

                // Send any queued messages
                this.sendQueuedMessages();

                // Perform identify/handshake first; subscriptions are sent after server Ready
                this.sendIdentify();

                // Notify connection callbacks
                this.connectionCallbacks.forEach(callback => callback(true));
            };

            this.ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    console.log('WebSocketManager: Received event:', data);

                    // New protocol: handshake + enveloped replay
                    if (data.type === 'ready' && data.snapshot) {
                        this.handleReady(data);
                        return;
                    }
                    if (data.type === 'event' && data.topic === 'compilation' && data.payload) {
                        // Treat replayed enveloped payloads like normal compilation events
                        this.handleCompilationEvent(data.payload);
                        // Track last seen
                        this.compilationLastSeq = typeof data.seq === 'number' ? data.seq : this.compilationLastSeq;
                        return;
                    }

                    // Handle file events
                    if (data.type === 'file_event' && data.event) {
                        this.handleFileEvent(data.event.event_type, data.event);
                    }

                    // Handle connection status
                    if (data.type === 'connection') {
                        console.log('WebSocketManager: Connection status:', data.status);
                    }

                    // Legacy compilation events
                    if (data.type === 'compilation_event') {
                        this.handleCompilationEvent(data.event);
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
        this.messageQueue = []; // Clear message queue

        // Notify connection callbacks
        this.connectionCallbacks.forEach(callback => callback(false));
    }

    // Send message to server
    send(data: any): void {
        if (this.isConnected && this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(data));
        } else {
            // Queue message for later if connecting
            if (this.ws && this.ws.readyState === WebSocket.CONNECTING) {
                console.log('WebSocketManager: WebSocket is still connecting, queuing message');
                this.messageQueue.push(data);
            } else {
                console.warn('WebSocketManager: Cannot send message - not connected and not connecting');
            }
        }
    }

    // Send all queued messages
    private sendQueuedMessages(): void {
        if (this.messageQueue.length > 0) {
            console.log(`WebSocketManager: Sending ${this.messageQueue.length} queued messages`);
            this.messageQueue.forEach(message => {
                if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                    this.ws.send(JSON.stringify(message));
                }
            });
            this.messageQueue = [];
        }
    }

    // ===== Handshake state =====
    private compilationLastSeq: number = 0;

    private sendIdentify(): void {
        const identify = {
            type: 'identify',
            lastSeen: {
                compilation: this.compilationLastSeq || 0,
            },
        };
        this.send(identify);
    }

    private handleReady(data: any): void {
        try {
            // Apply snapshot into existing consumers via synthetic events
            const snapshot = data.snapshot;
            if (snapshot?.latex) {
                const latex = snapshot.latex;
                // Emit synthetic events to initialize state for legacy consumers
                if (latex.main_file) {
                    this.handleCompilationEvent({
                        id: 'snapshot-main-file',
                        event_type: 'main_file_detected',
                        main_file: latex.main_file,
                        timestamp: Date.now(),
                    });
                }
                switch (latex.phase) {
                    case 'queued':
                        this.handleCompilationEvent({
                            id: 'snapshot-queued',
                            event_type: 'queued',
                            main_file: latex.main_file || '',
                            timestamp: Date.now(),
                            metadata: { reason: 'snapshot' },
                        });
                        break;
                    case 'started':
                        this.handleCompilationEvent({
                            id: 'snapshot-started',
                            event_type: 'started',
                            main_file: latex.main_file || '',
                            timestamp: Date.now(),
                            metadata: { engine: latex.engine || undefined },
                        });
                        break;
                    case 'success':
                        this.handleCompilationEvent({
                            id: 'snapshot-success',
                            event_type: 'success',
                            main_file: latex.main_file || '',
                            timestamp: Date.now(),
                            metadata: {
                                pdf_path: latex.pdf_path || undefined,
                                duration_ms: 0,
                                engine: latex.engine || undefined,
                            },
                        });
                        break;
                    case 'error':
                        this.handleCompilationEvent({
                            id: 'snapshot-error',
                            event_type: 'error',
                            main_file: latex.main_file || '',
                            timestamp: Date.now(),
                            metadata: { errors: latex.errors || ['error'] },
                        });
                        break;
                }
            }

            // Update cursors and send legacy subscriptions for compatibility
            if (typeof data.cursors?.compilation === 'number') {
                this.compilationLastSeq = data.cursors.compilation;
            }
            this.handleConnectionOpen();
        } catch (e) {
            console.error('WebSocketManager: Failed to handle Ready message:', e);
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

        // Ensure connection is established
        if (!this.ws || (this.ws.readyState !== WebSocket.OPEN && this.ws.readyState !== WebSocket.CONNECTING)) {
            this.connect();
        }

        // Drain buffered events to this new subscriber
        if (this.compilationEventBuffer.length > 0) {
            try {
                this.compilationEventBuffer.forEach(evt => {
                    try { callback(evt); } catch (e) { console.error('WebSocketManager: Error delivering buffered event:', e); }
                });
            } catch (e) {
                console.error('WebSocketManager: Failed draining buffered compilation events:', e);
            }
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
        // Buffer for late subscribers
        this.compilationEventBuffer.push(event);
        if (this.compilationEventBuffer.length > this.maxCompilationBuffer) {
            this.compilationEventBuffer.shift();
        }
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