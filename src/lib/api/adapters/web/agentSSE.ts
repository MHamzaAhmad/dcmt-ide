// Agent SSE (Server-Sent Events) Adapter for Real-time Agent Events
import type { AgentEvent } from '../../types';
import { eventStore } from '$lib/stores/events';

export type AgentEventCallback = (event: AgentEvent) => void;

/**
 * SSE adapter for agent events - simpler and more reliable than WebSockets
 * This replaces the WebSocket implementation for agent events
 */
export class AgentSSEAdapter {
    private eventSource: EventSource | null = null;
    private sessionCallbacks: Map<string, AgentEventCallback[]> = new Map();
    private globalCallbacks: AgentEventCallback[] = [];
    private reconnectAttempts = 0;
    private maxReconnectAttempts = 5;
    private reconnectDelay = 1000;
    private isDestroyed = false;
    private currentSessionId: string | null = null;
    private reconnectTimeout: number | null = null;

    /**
     * Connect to SSE endpoint for a specific session
     */
    connectToSession(sessionId: string): void {
        if (this.isDestroyed) return;
        
        // Disconnect from previous session if any
        this.disconnect();
        
        this.currentSessionId = sessionId;
        // Use relative URL to let nginx handle proxying in Docker deployment
        const baseUrl = import.meta.env.VITE_API_BASE_URL || '/sse';
        const sseUrl = `${baseUrl}/agent/session/${sessionId}/events`;

        console.log('Connecting to agent SSE for session:', sessionId, 'URL:', sseUrl);
        
        try {
            this.eventSource = new EventSource(sseUrl);
            
            this.eventSource.onopen = () => {
                console.log('Agent SSE connected for session:', sessionId);
                this.reconnectAttempts = 0;
                this.reconnectDelay = 1000;
                
                // Update connection status
                eventStore.updateConnectionStatus('websocket', 'connected');
            };
            
            this.eventSource.addEventListener('agent-event', (event) => {
                try {
                    const data = JSON.parse(event.data);
                    console.log('Agent SSE event received:', data);
                    this.handleAgentEvent(data);
                } catch (error) {
                    console.error('Failed to parse SSE agent event:', error, 'Raw data:', event.data);
                }
            });
            
            this.eventSource.addEventListener('error', (event: MessageEvent) => {
                // Handle custom error events from server
                try {
                    const errorData = JSON.parse(event.data);
                    console.error('Agent SSE error event:', errorData);
                } catch {
                    // Not a JSON error event
                }
            });
            
            this.eventSource.onerror = (error) => {
                console.error('Agent SSE error:', error);
                
                // Update connection status
                eventStore.updateConnectionStatus('websocket', 'error');
                
                // EventSource will automatically reconnect, but we can add our own logic
                if (this.eventSource?.readyState === EventSource.CLOSED) {
                    console.log('SSE connection closed, attempting reconnect...');
                    this.attemptReconnect();
                }
            };
            
        } catch (error) {
            console.error('Failed to create agent SSE connection:', error);
            this.attemptReconnect();
        }
    }

    /**
     * Connect to global SSE endpoint (all events)
     */
    connectGlobal(): void {
        if (this.isDestroyed) return;
        
        // Disconnect from any existing connection
        this.disconnect();
        
        this.currentSessionId = null;
        // Use relative URL to let nginx handle proxying in Docker deployment
        const sseUrl = `/sse/agent/events`;
        
        console.log('Connecting to global agent SSE, URL:', sseUrl);
        
        try {
            this.eventSource = new EventSource(sseUrl);
            
            this.eventSource.onopen = () => {
                console.log('Global agent SSE connected');
                this.reconnectAttempts = 0;
                this.reconnectDelay = 1000;
                
                // Update connection status
                eventStore.updateConnectionStatus('websocket', 'connected');
            };
            
            this.eventSource.addEventListener('agent-event', (event) => {
                try {
                    const data = JSON.parse(event.data);
                    console.log('Global agent SSE event received:', data);
                    this.handleAgentEvent(data);
                } catch (error) {
                    console.error('Failed to parse global SSE agent event:', error, 'Raw data:', event.data);
                }
            });
            
            this.eventSource.onerror = (error) => {
                console.error('Global agent SSE error:', error);
                
                // Update connection status
                eventStore.updateConnectionStatus('websocket', 'error');
                
                // EventSource will automatically reconnect
                if (this.eventSource?.readyState === EventSource.CLOSED) {
                    console.log('Global SSE connection closed, attempting reconnect...');
                    this.attemptReconnect();
                }
            };
            
        } catch (error) {
            console.error('Failed to create global agent SSE connection:', error);
            this.attemptReconnect();
        }
    }

    private attemptReconnect(): void {
        if (this.isDestroyed || this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.error('Max reconnection attempts reached for agent SSE');
            return;
        }

        this.reconnectAttempts++;
        const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1); // Exponential backoff
        
        console.log(`Attempting to reconnect agent SSE in ${delay}ms (attempt ${this.reconnectAttempts})`);
        
        if (this.reconnectTimeout) {
            clearTimeout(this.reconnectTimeout);
        }
        
        this.reconnectTimeout = window.setTimeout(() => {
            if (!this.isDestroyed) {
                if (this.currentSessionId) {
                    this.connectToSession(this.currentSessionId);
                } else {
                    this.connectGlobal();
                }
            }
        }, delay);
    }

    private handleAgentEvent(event: AgentEvent): void {
        console.log('Processing agent event:', event.type, event);
        
        // Get session ID from event if available, or use current session
        let sessionId: string | null = null;
        if ('session_id' in event) {
            sessionId = event.session_id;
        } else {
            // For events without session_id, use the current session we're connected to
            sessionId = this.currentSessionId;
        }

        // Emit to EventStore
        if (sessionId) {
            this.emitToEventStore(event, sessionId);
        }

        // Call global callbacks
        this.globalCallbacks.forEach(callback => {
            try {
                callback(event);
            } catch (error) {
                console.error(`Error in global agent event callback:`, error);
            }
        });

        // Call session-specific callbacks if we have a session ID
        if (sessionId) {
            const sessionCallbacks = this.sessionCallbacks.get(sessionId) || [];
            console.log(`Calling ${sessionCallbacks.length} session callbacks for session ${sessionId}`);
            sessionCallbacks.forEach(callback => {
                try {
                    callback(event);
                } catch (error) {
                    console.error(`Error in session agent event callback for ${sessionId}:`, error);
                }
            });
        } else {
            console.log('No session ID found for event, skipping session callbacks');
        }
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

    /**
     * Subscribe to events for a specific session
     */
    onSession(sessionId: string, callback: AgentEventCallback): () => void {
        if (!this.sessionCallbacks.has(sessionId)) {
            this.sessionCallbacks.set(sessionId, []);
        }

        const callbacks = this.sessionCallbacks.get(sessionId)!;
        callbacks.push(callback);

        // If not connected or connected to a different session, reconnect
        if (!this.eventSource || this.currentSessionId !== sessionId) {
            this.connectToSession(sessionId);
        }

        // Return unsubscribe function
        return () => {
            const index = callbacks.indexOf(callback);
            if (index > -1) {
                callbacks.splice(index, 1);
            }

            // If no more callbacks for any session, disconnect
            if (callbacks.length === 0) {
                this.sessionCallbacks.delete(sessionId);
                if (this.sessionCallbacks.size === 0 && this.globalCallbacks.length === 0) {
                    this.disconnect();
                }
            }
        };
    }

    /**
     * Subscribe to all agent events
     */
    onAny(callback: AgentEventCallback): () => void {
        this.globalCallbacks.push(callback);

        // If not connected, connect to global endpoint
        if (!this.eventSource) {
            this.connectGlobal();
        }

        // Return unsubscribe function
        return () => {
            const index = this.globalCallbacks.indexOf(callback);
            if (index > -1) {
                this.globalCallbacks.splice(index, 1);
            }

            // If no more callbacks, disconnect
            if (this.globalCallbacks.length === 0 && this.sessionCallbacks.size === 0) {
                this.disconnect();
            }
        };
    }

    /**
     * Disconnect from SSE
     */
    disconnect(): void {
        if (this.eventSource) {
            console.log('Disconnecting agent SSE');
            this.eventSource.close();
            this.eventSource = null;
            
            // Update connection status
            eventStore.updateConnectionStatus('websocket', 'disconnected');
        }
        
        if (this.reconnectTimeout) {
            clearTimeout(this.reconnectTimeout);
            this.reconnectTimeout = null;
        }
    }

    /**
     * Check if connected
     */
    isConnected(): boolean {
        return this.eventSource !== null && this.eventSource.readyState === EventSource.OPEN;
    }

    /**
     * Get connection status
     */
    getStatus(): {
        connected: boolean;
        reconnectAttempts: number;
        sessionId: string | null;
    } {
        return {
            connected: this.isConnected(),
            reconnectAttempts: this.reconnectAttempts,
            sessionId: this.currentSessionId
        };
    }

    /**
     * Destroy the adapter
     */
    async destroy(): Promise<void> {
        console.log('Destroying agent SSE adapter');
        this.isDestroyed = true;
        
        // Clear all callbacks
        this.sessionCallbacks.clear();
        this.globalCallbacks.length = 0;
        
        // Disconnect
        this.disconnect();
    }
}

// Export singleton instance
export const agentSSE = new AgentSSEAdapter();