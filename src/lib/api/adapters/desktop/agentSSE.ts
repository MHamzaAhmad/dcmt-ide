// Agent SSE (Server-Sent Events) Adapter for Desktop platform using fetch streaming
// Note: Using standard fetch API as Tauri v2 supports it natively
import type { AgentEvent } from '../../types';
import { eventStore } from '$lib/stores/events';

export type AgentEventCallback = (event: AgentEvent) => void;

/**
 * SSE adapter for agent events on desktop platform
 * Uses Tauri's fetch API with streaming to mimic EventSource behavior
 */
export class DesktopAgentSSEAdapter {
    private abortController: AbortController | null = null;
    private sessionCallbacks: Map<string, AgentEventCallback[]> = new Map();
    private globalCallbacks: AgentEventCallback[] = [];
    private reconnectAttempts = 0;
    private maxReconnectAttempts = 5;
    private reconnectDelay = 1000;
    private isDestroyed = false;
    private currentSessionId: string | null = null;
    private reconnectTimeout: number | null = null;
    private isConnecting = false;

    /**
     * Connect to SSE endpoint for a specific session
     */
    async connectToSession(sessionId: string): Promise<void> {
        if (this.isDestroyed || this.isConnecting) return;
        
        // Disconnect from previous session if any
        this.disconnect();
        
        this.currentSessionId = sessionId;
        // Construct the SSE URL - need to get backend URL from environment or config
        const backendUrl = import.meta.env.VITE_BACKEND_URL || 'http://localhost:3000';
        const sseUrl = `${backendUrl}/sse/agent/session/${sessionId}/events`;
        
        console.log('Connecting to agent SSE for session:', sessionId, 'URL:', sseUrl);
        
        this.isConnecting = true;
        
        try {
            this.abortController = new AbortController();
            
            const response = await fetch(sseUrl, {
                headers: {
                    'Accept': 'text/event-stream',
                    'Cache-Control': 'no-cache',
                },
                signal: this.abortController.signal,
            });

            if (!response.ok) {
                throw new Error(`SSE request failed: ${response.status} ${response.statusText}`);
            }

            console.log('Agent SSE connected for session:', sessionId);
            this.reconnectAttempts = 0;
            this.reconnectDelay = 1000;
            this.isConnecting = false;
            
            // Update connection status
            eventStore.updateConnectionStatus('tauri', 'connected');

            // Start reading the stream
            await this.processSSEStream(response);
            
        } catch (error: any) {
            this.isConnecting = false;
            
            if (error.name === 'AbortError') {
                console.log('Agent SSE connection aborted');
                return;
            }
            
            console.error('Failed to connect to agent SSE:', error);
            eventStore.updateConnectionStatus('tauri', 'error');
            this.attemptReconnect();
        }
    }

    /**
     * Connect to global SSE endpoint (all events)
     */
    async connectGlobal(): Promise<void> {
        if (this.isDestroyed || this.isConnecting) return;
        
        // Disconnect from any existing connection
        this.disconnect();
        
        this.currentSessionId = null;
        const backendUrl = import.meta.env.VITE_BACKEND_URL || 'http://localhost:3000';
        const sseUrl = `${backendUrl}/sse/agent/events`;
        
        console.log('Connecting to global agent SSE, URL:', sseUrl);
        
        this.isConnecting = true;
        
        try {
            this.abortController = new AbortController();
            
            const response = await fetch(sseUrl, {
                headers: {
                    'Accept': 'text/event-stream',
                    'Cache-Control': 'no-cache',
                },
                signal: this.abortController.signal,
            });

            if (!response.ok) {
                throw new Error(`SSE request failed: ${response.status} ${response.statusText}`);
            }

            console.log('Global agent SSE connected');
            this.reconnectAttempts = 0;
            this.reconnectDelay = 1000;
            this.isConnecting = false;
            
            // Update connection status
            eventStore.updateConnectionStatus('tauri', 'connected');

            // Start reading the stream
            await this.processSSEStream(response);
            
        } catch (error: any) {
            this.isConnecting = false;
            
            if (error.name === 'AbortError') {
                console.log('Global agent SSE connection aborted');
                return;
            }
            
            console.error('Failed to connect to global agent SSE:', error);
            eventStore.updateConnectionStatus('tauri', 'error');
            this.attemptReconnect();
        }
    }

    /**
     * Process the SSE stream from the response
     */
    private async processSSEStream(response: Response): Promise<void> {
        if (!response.body) {
            throw new Error('Response body is null');
        }

        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';

        try {
            while (true) {
                const { done, value } = await reader.read();
                
                if (done) {
                    console.log('SSE stream ended');
                    break;
                }

                buffer += decoder.decode(value, { stream: true });
                
                // Process complete lines
                const lines = buffer.split('\n');
                buffer = lines.pop() || ''; // Keep incomplete line in buffer

                this.parseSSELines(lines);
            }
        } catch (error: any) {
            if (error.name === 'AbortError') {
                console.log('SSE stream reading aborted');
            } else {
                console.error('Error reading SSE stream:', error);
                throw error;
            }
        } finally {
            reader.releaseLock();
        }
    }

    /**
     * Parse SSE lines and extract events
     */
    private parseSSELines(lines: string[]): void {
        let eventType = '';
        let data = '';

        for (const line of lines) {
            if (line.startsWith('event:')) {
                eventType = line.substring(6).trim();
            } else if (line.startsWith('data:')) {
                data += line.substring(5).trim();
            } else if (line === '') {
                // Empty line indicates end of event
                if (eventType && data) {
                    this.handleSSEEvent(eventType, data);
                }
                eventType = '';
                data = '';
            }
        }
    }

    /**
     * Handle a parsed SSE event
     */
    private handleSSEEvent(eventType: string, data: string): void {
        if (eventType === 'agent-event') {
            try {
                const agentEvent = JSON.parse(data);
                console.log('Desktop agent SSE event received:', agentEvent);
                this.handleAgentEvent(agentEvent);
            } catch (error) {
                console.error('Failed to parse SSE agent event:', error, 'Raw data:', data);
            }
        } else if (eventType === 'error') {
            console.error('SSE error event:', data);
        } else if (eventType === 'ping') {
            // Keep-alive ping, ignore
        } else {
            console.log('Unknown SSE event type:', eventType, 'Data:', data);
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
        
        this.reconnectTimeout = window.setTimeout(async () => {
            if (!this.isDestroyed) {
                if (this.currentSessionId) {
                    await this.connectToSession(this.currentSessionId);
                } else {
                    await this.connectGlobal();
                }
            }
        }, delay);
    }

    private handleAgentEvent(event: AgentEvent): void {
        console.log('Processing agent event:', event.type, event);
        
        // Get session ID from event if available
        let sessionId: string | null = null;
        if ('session_id' in event) {
            sessionId = event.session_id;
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
            sessionCallbacks.forEach(callback => {
                try {
                    callback(event);
                } catch (error) {
                    console.error(`Error in session agent event callback for ${sessionId}:`, error);
                }
            });
        }
    }

    private emitToEventStore(event: AgentEvent, sessionId: string): void {
        // Map agent event types to EventStore types - same as web adapter
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
        if (!this.isConnected() || this.currentSessionId !== sessionId) {
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
        if (!this.isConnected()) {
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
        if (this.abortController) {
            console.log('Disconnecting desktop agent SSE');
            this.abortController.abort();
            this.abortController = null;
            
            // Update connection status
            eventStore.updateConnectionStatus('tauri', 'disconnected');
        }
        
        if (this.reconnectTimeout) {
            clearTimeout(this.reconnectTimeout);
            this.reconnectTimeout = null;
        }
        
        this.isConnecting = false;
    }

    /**
     * Check if connected
     */
    isConnected(): boolean {
        return this.abortController !== null && !this.isConnecting;
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
        console.log('Destroying desktop agent SSE adapter');
        this.isDestroyed = true;
        
        // Clear all callbacks
        this.sessionCallbacks.clear();
        this.globalCallbacks.length = 0;
        
        // Disconnect
        this.disconnect();
    }
}

// Export singleton instance
export const desktopAgentSSE = new DesktopAgentSSEAdapter();