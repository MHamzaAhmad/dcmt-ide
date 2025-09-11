// Agent WebSocket Adapter for Real-time Agent Events
import type { AgentEvent, FileEventData } from '../../types';
import type { FileEventCallback, FileEventType } from '../../hooks/useFileWatcher';
import { eventStore } from '$lib/stores/events';

export type AgentEventCallback = (event: AgentEvent) => void;

export class AgentWebSocketAdapter {
    private ws: WebSocket | null = null;
    private eventCallbacks: Map<string, AgentEventCallback[]> = new Map();
    private sessionCallbacks: Map<string, AgentEventCallback[]> = new Map();
    private fileEventCallbacks: FileEventCallback[] = [];
    private reconnectAttempts = 0;
    private maxReconnectAttempts = 5;
    private reconnectDelay = 1000; // Start with 1s delay
    private isConnecting = false;
    private isDestroyed = false;
    private subscribedSessions: Set<string> = new Set();
    private pendingMessages: any[] = []; // Queue for messages when not connected

    constructor() {
        this.connect();
    }

    private connect(): void {
        if (this.isConnecting || this.isDestroyed) {
            return;
        }

        this.isConnecting = true;

        try {
            // Use relative URL to let nginx handle proxying in Docker deployment
            // Construct WebSocket URL based on current protocol and host
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${import.meta.env.VITE_API_BASE_URL}/ws`;
            console.log('Connecting to agent WebSocket:', wsUrl);
            
            this.ws = new WebSocket(wsUrl);

            this.ws.onopen = () => {
                console.log('Agent WebSocket connected');
                this.isConnecting = false;
                this.reconnectAttempts = 0;
                this.reconnectDelay = 1000;
                
                // Update connection status in EventStore
                eventStore.updateConnectionStatus('websocket', 'connected');
                
                // Resubscribe to all sessions and file events after reconnection
                this.resubscribeToSessions();
            };

            this.ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    console.log('Agent WebSocket message received:', data);
                    
                    // Handle different message formats
                    if (data.type === 'agent_event' && data.event) {
                        // Wrapped format: {type: 'agent_event', event: {...}}
                        console.log('Handling wrapped agent event:', data.event.type);
                        this.handleAgentEvent(data.event);
                    } else if (data.type && this.isAgentEventType(data.type)) {
                        // Direct agent event format
                        console.log('Handling direct agent event:', data.type);
                        this.handleAgentEvent(data as AgentEvent);
                    } else if (data.type === 'file_event' && data.event_data) {
                        // File event format
                        console.log('Handling file event:', data.event_type);
                        this.handleFileEvent(data.event_type, data.event_data);
                    } else {
                        console.warn('Unknown WebSocket message format:', data);
                    }
                } catch (error) {
                    console.error('Failed to parse agent WebSocket message:', error, 'Raw message:', event.data);
                }
            };

            this.ws.onerror = (error) => {
                console.error('Agent WebSocket error:', error);
                this.isConnecting = false;
                
                // Update connection status in EventStore
                eventStore.updateConnectionStatus('websocket', 'error');
            };

            this.ws.onclose = (event) => {
                console.log('Agent WebSocket disconnected:', event.code, event.reason);
                this.ws = null;
                this.isConnecting = false;

                // Update connection status in EventStore
                eventStore.updateConnectionStatus('websocket', 'disconnected');

                // Attempt to reconnect if not manually closed and not destroyed
                if (!this.isDestroyed && event.code !== 1000) {
                    this.attemptReconnect();
                }
            };

        } catch (error) {
            console.error('Failed to create agent WebSocket:', error);
            this.isConnecting = false;
            this.attemptReconnect();
        }
    }

    private attemptReconnect(): void {
        if (this.isDestroyed || this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.error('Max reconnection attempts reached for agent WebSocket');
            return;
        }

        this.reconnectAttempts++;
        const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1); // Exponential backoff
        
        console.log(`Attempting to reconnect agent WebSocket in ${delay}ms (attempt ${this.reconnectAttempts})`);
        
        setTimeout(() => {
            if (!this.isDestroyed) {
                this.connect();
            }
        }, delay);
    }

    private isAgentEventType(type: string): boolean {
        const eventTypes = [
            'JobQueued', 'LLMCallStart', 'LLMStreaming', 'ToolCallRequested',
            'ToolExecuting', 'ToolCompleted', 'ParallelToolsStart', 
            'ParallelToolsComplete', 'LLMCallComplete', 'JobComplete', 'Error'
        ];
        return eventTypes.includes(type);
    }

    private emitToEventStore(event: AgentEvent, sessionId: string): void {
        // Map agent event types to EventStore types
        const typeMapping: Record<string, any> = {
            'JobQueued': 'job_queued',
            'LLMCallStart': 'llm_call_start', 
            'LLMStreaming': 'llm_streaming',
            'ToolCallRequested': 'tool_call_requested',
            'ToolExecuting': 'tool_executing',
            'ToolCompleted': 'tool_completed',
            'ParallelToolsStart': 'parallel_tools_start',
            'ParallelToolsComplete': 'parallel_tools_complete',
            'LLMCallComplete': 'llm_call_complete',
            'JobComplete': 'job_complete',
            'Error': 'error'
        };

        const subtype = typeMapping[event.type];
        if (!subtype) return;

        // Extract relevant data from event
        const payload: any = { sessionId };
        
        if ('tool' in event) payload.tool = event.tool;
        if ('result' in event) payload.result = event.result;
        if ('message' in event) payload.message = event.message;

        eventStore.emit({
            type: 'agent',
            subtype,
            payload
        });
    }

    private handleAgentEvent(event: AgentEvent): void {
        console.log('Processing agent event:', event.type, event);
        
        // Get session ID from event if available
        let sessionId: string | null = null;
        if ('session_id' in event) {
            sessionId = event.session_id;
        }

        // Fallback: For events without session_id, use the first subscribed session
        // This handles cases where JobComplete events don't include session context
        if (!sessionId && this.subscribedSessions.size > 0) {
            sessionId = Array.from(this.subscribedSessions)[0];
            console.log(`AgentWebSocket: Using fallback session ${sessionId} for ${event.type}`);
        }

        console.log(`AgentWebSocket: SessionId for ${event.type}: ${sessionId}`);

        // Emit to EventStore - ONLY emit raw agent events
        // Do NOT convert to file system events here - that's agent.ts responsibility
        if (sessionId) {
            console.log(`AgentWebSocket: Emitting ${event.type} to EventStore with session ${sessionId}`);
            this.emitToEventStore(event, sessionId);
        } else {
            console.warn(`AgentWebSocket: No session_id found for ${event.type} event, cannot emit to EventStore`);
        }

        // Call global event callbacks
        const globalCallbacks = this.eventCallbacks.get(event.type) || [];
        globalCallbacks.forEach(callback => {
            try {
                callback(event);
            } catch (error) {
                console.error(`Error in agent event callback for ${event.type}:`, error);
            }
        });

        // Call session-specific callbacks
        if (sessionId) {
            const sessionCallbacks = this.sessionCallbacks.get(sessionId) || [];
            sessionCallbacks.forEach(callback => {
                try {
                    callback(event);
                } catch (error) {
                    console.error(`Error in session agent event callback for ${sessionId}:`, error);
                }
            });
        }
    }

    // All file operation handling removed - this is now handled by agent.ts only

    /**
     * Subscribe to specific agent event types
     */
    on(eventType: AgentEvent['type'], callback: AgentEventCallback): () => void {
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
     * Subscribe to all agent events
     */
    onAny(callback: AgentEventCallback): () => void {
        // Subscribe to all known event types
        const eventTypes: AgentEvent['type'][] = [
            'JobQueued', 'LLMCallStart', 'LLMStreaming', 'ToolCallRequested',
            'ToolExecuting', 'ToolCompleted', 'ParallelToolsStart', 
            'ParallelToolsComplete', 'LLMCallComplete', 'JobComplete', 'Error'
        ];

        const unsubscribers = eventTypes.map(eventType => this.on(eventType, callback));

        return () => {
            unsubscribers.forEach(unsub => unsub());
        };
    }

    /**
     * Subscribe to events for a specific session
     */
    onSession(sessionId: string, callback: AgentEventCallback): () => void {
        if (!this.sessionCallbacks.has(sessionId)) {
            this.sessionCallbacks.set(sessionId, []);
        }

        const callbacks = this.sessionCallbacks.get(sessionId)!;
        callbacks.push(callback);

        // Send subscription message to server
        this.send({
            type: 'subscribe_agent',
            session_id: sessionId
        });
        
        // Track subscription for reconnection
        this.subscribedSessions.add(sessionId);

        // Return unsubscribe function
        return () => {
            const index = callbacks.indexOf(callback);
            if (index > -1) {
                callbacks.splice(index, 1);
            }

            // If no more callbacks for this session, unsubscribe
            if (callbacks.length === 0) {
                this.sessionCallbacks.delete(sessionId);
                this.subscribedSessions.delete(sessionId);
                
                if (this.isConnected()) {
                    this.send({
                        type: 'unsubscribe_agent',
                        session_id: sessionId
                    });
                }
            }
        };
    }

    /**
     * Check if WebSocket is connected
     */
    isConnected(): boolean {
        return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
    }

    /**
     * Send message to server
     */
    send(data: any): void {
        if (this.isConnected() && this.ws) {
            const message = JSON.stringify(data);
            console.log('Sending WebSocket message:', data);
            this.ws.send(message);
        } else {
            console.warn('Cannot send message: agent WebSocket not connected, queueing for later', data);
            this.pendingMessages.push(data);
        }
    }

    /**
     * Manually reconnect
     */
    reconnect(): void {
        if (this.ws) {
            this.ws.close();
        }
        this.reconnectAttempts = 0;
        this.connect();
    }

    /**
     * Close connection and clean up
     */
    async destroy(): Promise<void> {
        console.log('Destroying agent WebSocket adapter');
        this.isDestroyed = true;
        
        // Clear all callbacks
        this.eventCallbacks.clear();
        this.sessionCallbacks.clear();
        this.fileEventCallbacks.length = 0;
        this.subscribedSessions.clear();

        // Close WebSocket connection
        if (this.ws) {
            this.ws.close(1000, 'Client destroying connection');
            this.ws = null;
        }
    }

    /**
     * Get connection status info
     */
    getStatus(): {
        connected: boolean;
        reconnectAttempts: number;
        isConnecting: boolean;
    } {
        return {
            connected: this.isConnected(),
            reconnectAttempts: this.reconnectAttempts,
            isConnecting: this.isConnecting
        };
    }

    /**
     * Handle file events from the WebSocket
     */
    private handleFileEvent(eventType: FileEventType, event: FileEventData): void {
        console.log(`File event received (${eventType}):`, event);
        
        // Only emit to EventStore for actual file watcher events, not agent operations
        // Agent operations are already emitted in handleFileOperation()
        const source = 'watcher'; // These are external file events
        switch (eventType) {
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
                    // Fallback to modified if no old path
                    eventStore.events.fileModified(event.path, event.metadata.size, source);
                }
                break;
        }
        
        // Call all file event callbacks
        this.fileEventCallbacks.forEach(callback => {
            try {
                callback(eventType, event);
            } catch (error) {
                console.error('Error in file event callback:', error);
            }
        });
    }

    /**
     * Subscribe to file events
     */
    onFileEvents(callback: FileEventCallback): () => void {
        this.fileEventCallbacks.push(callback);
        
        // Send subscription message for file events if connected
        if (this.isConnected()) {
            this.send({
                type: 'subscribe_files'
            });
        }
        
        // Return unsubscribe function
        return () => {
            const index = this.fileEventCallbacks.indexOf(callback);
            if (index > -1) {
                this.fileEventCallbacks.splice(index, 1);
            }
            
            // If no more file callbacks, unsubscribe from file events
            if (this.fileEventCallbacks.length === 0 && this.isConnected()) {
                this.send({
                    type: 'unsubscribe_files'
                });
            }
        };
    }

    /**
     * Re-subscribe to all sessions after reconnection
     */
    private resubscribeToSessions(): void {
        console.log('Resubscribing to sessions:', this.subscribedSessions.size, 'sessions');
        
        // Send any pending messages first
        if (this.pendingMessages.length > 0) {
            console.log('Sending', this.pendingMessages.length, 'pending messages');
            const messages = [...this.pendingMessages];
            this.pendingMessages = [];
            messages.forEach(msg => this.send(msg));
        }
        
        // Re-subscribe to agent sessions
        this.subscribedSessions.forEach(sessionId => {
            console.log('Resubscribing to session:', sessionId);
            this.send({
                type: 'subscribe_agent',
                session_id: sessionId
            });
        });
        
        // Re-subscribe to file events if we have callbacks
        if (this.fileEventCallbacks.length > 0) {
            console.log('Resubscribing to file events');
            this.send({
                type: 'subscribe_files'
            });
        }
        
        console.log('Re-subscription complete');
    }

    /**
     * Subscribe to both agent and file events for a session
     * This is a convenience method for the unified hook
     */
    onMixedEvents(
        sessionId: string,
        agentCallback: AgentEventCallback,
        fileCallback: FileEventCallback
    ): () => void {
        const agentUnsub = this.onSession(sessionId, agentCallback);
        const fileUnsub = this.onFileEvents(fileCallback);
        
        return () => {
            agentUnsub();
            fileUnsub();
        };
    }
}

// Export singleton instance
export const agentWebSocket = new AgentWebSocketAdapter();