// Agent WebSocket Adapter for Real-time Agent Events
import { apiClient } from '../../client';
import type { AgentEvent, FileEventData } from '../../types';
import type { FileEventCallback, FileEventType } from '../../hooks/useFileWatcher';

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
            const wsUrl = apiClient.baseURL.replace(/^http/, 'ws') + '/ws';
            console.log('Connecting to agent WebSocket:', wsUrl);
            
            this.ws = new WebSocket(wsUrl);

            this.ws.onopen = () => {
                console.log('Agent WebSocket connected');
                this.isConnecting = false;
                this.reconnectAttempts = 0;
                this.reconnectDelay = 1000;
                
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

    private isAgentEventType(type: string): boolean {
        const eventTypes = [
            'JobQueued', 'LLMCallStart', 'LLMStreaming', 'ToolCallRequested',
            'ToolExecuting', 'ToolCompleted', 'ParallelToolsStart', 
            'ParallelToolsComplete', 'LLMCallComplete', 'JobComplete', 'Error'
        ];
        return eventTypes.includes(type);
    }

    private handleAgentEvent(event: AgentEvent): void {
        console.log('Processing agent event:', event.type, event);
        
        // Get session ID from event if available
        let sessionId: string | null = null;
        if ('session_id' in event) {
            sessionId = event.session_id;
        }

        // Handle file operations immediately
        if (event.type === 'ToolCompleted' && this.isFileOperation(event.tool)) {
            this.handleFileOperation(event);
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

    private isFileOperation(tool: string): boolean {
        const fileOperations = [
            'read_file', 'write_file', 'update_file', 'create_file', 
            'delete_file', 'create_directory', 'rename_file', 'copy_file', 'move_file'
        ];
        return fileOperations.includes(tool);
    }

    private handleFileOperation(event: { tool: string; result: string }): void {
        console.log(`Detected file operation: ${event.tool}`, event.result);
        
        // Create a synthetic file event to trigger file system updates
        const eventType = this.getFileEventType(event.tool);
        const fileEventData: FileEventData = {
            event_type: eventType,
            path: this.extractPathFromResult(event.result) || 'unknown',
            timestamp: Date.now(),
            metadata: {
                is_directory: event.tool === 'create_directory',
                size: undefined,
                old_path: undefined,
                new_path: undefined
            }
        };

        // Trigger file event handling - convert to FileEventType
        const fileEventType = eventType.toLowerCase() as FileEventType;
        this.handleFileEvent(fileEventType, fileEventData);
    }

    private getFileEventType(tool: string): 'Created' | 'Modified' | 'Deleted' | 'Renamed' {
        switch (tool) {
            case 'create_file':
            case 'create_directory':
                return 'Created';
            case 'write_file':
            case 'update_file':
                return 'Modified';
            case 'delete_file':
                return 'Deleted';
            case 'rename_file':
            case 'move_file':
                return 'Renamed';
            default:
                return 'Modified';
        }
    }

    private extractPathFromResult(result: string): string | null {
        try {
            const parsed = JSON.parse(result);
            return parsed.path || parsed.file_path || parsed.filename || null;
        } catch {
            // Extract from text patterns
            const patterns = [
                /Successfully (?:wrote|created|updated|deleted) ['""]?([^'""]+)['""]?/i,
                /(?:file|path)[:=]\s*['""]?([^'""]+)['""]?/i,
                /['""]([\/\w\-\.]+\.\w+)['""]/ // Files with extensions
            ];
            
            for (const pattern of patterns) {
                const match = result.match(pattern);
                if (match && match[1]) {
                    return match[1].trim();
                }
            }
        }
        return null;
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
        
        // Call all file event callbacks
        this.fileEventCallbacks.forEach(callback => {
            try {
                callback(eventType, event);
            } catch (error) {
                console.error('Error in file event callback:', error);
            }
        });
        
        // Emit as browser event for compatibility
        if (typeof window !== 'undefined') {
            window.dispatchEvent(new CustomEvent('file-event', { 
                detail: { eventType, event } 
            }));
        }
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