// Agent WebSocket Adapter for Real-time Agent Events
import { apiClient } from '../../client';
import type { AgentEvent } from '../../types';

export type AgentEventCallback = (event: AgentEvent) => void;

export class AgentWebSocketAdapter {
    private ws: WebSocket | null = null;
    private eventCallbacks: Map<string, AgentEventCallback[]> = new Map();
    private sessionCallbacks: Map<string, AgentEventCallback[]> = new Map();
    private reconnectAttempts = 0;
    private maxReconnectAttempts = 5;
    private reconnectDelay = 1000; // Start with 1s delay
    private isConnecting = false;
    private isDestroyed = false;

    constructor() {
        this.connect();
    }

    private connect(): void {
        if (this.isConnecting || this.isDestroyed) {
            return;
        }

        this.isConnecting = true;

        try {
            const wsUrl = apiClient.baseURL.replace(/^http/, 'ws') + '/ws';
            console.log('Connecting to agent WebSocket:', wsUrl);
            
            this.ws = new WebSocket(wsUrl);

            this.ws.onopen = () => {
                console.log('Agent WebSocket connected');
                this.isConnecting = false;
                this.reconnectAttempts = 0;
                this.reconnectDelay = 1000;
            };

            this.ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    console.log('Agent WebSocket event:', data);
                    
                    // Handle agent-specific events
                    if (data.type === 'agent_event' && data.event) {
                        this.handleAgentEvent(data.event);
                    }
                } catch (error) {
                    console.error('Failed to parse agent WebSocket message:', error);
                }
            };

            this.ws.onerror = (error) => {
                console.error('Agent WebSocket error:', error);
                this.isConnecting = false;
            };

            this.ws.onclose = (event) => {
                console.log('Agent WebSocket disconnected:', event.code, event.reason);
                this.ws = null;
                this.isConnecting = false;

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

    private handleAgentEvent(event: AgentEvent): void {
        // Get session ID from event if available
        let sessionId: string | null = null;
        if ('session_id' in event) {
            sessionId = event.session_id;
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

        // Emit as browser event for compatibility
        if (typeof window !== 'undefined') {
            window.dispatchEvent(new CustomEvent('agent-event', { 
                detail: { event, sessionId } 
            }));
        }
    }

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

        // Send subscription message to server if connected
        if (this.isConnected()) {
            this.send({
                type: 'subscribe_agent',
                session_id: sessionId
            });
        }

        // Return unsubscribe function
        return () => {
            const index = callbacks.indexOf(callback);
            if (index > -1) {
                callbacks.splice(index, 1);
            }

            // If no more callbacks for this session, unsubscribe
            if (callbacks.length === 0) {
                this.sessionCallbacks.delete(sessionId);
                
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
            this.ws.send(JSON.stringify(data));
        } else {
            console.warn('Cannot send message: agent WebSocket not connected');
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
}

// Export singleton instance
export const agentWebSocket = new AgentWebSocketAdapter();