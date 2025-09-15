// Desktop Agent Operations via Tauri Commands
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { modelsAPI } from '../../models';
import type {
    AgentOperations,
    AgentChatRequest,
    AgentChatResponse,
    AgentSessionInfo,
    AgentToolDefinition,
    LiteLLMModelsResponse,
    AgentEvent
} from '../../types';

export type AgentEventCallback = (event: AgentEvent) => void;
export type FileEventCallback = (type: string, event: any) => void;

export class DesktopAgentAdapter implements AgentOperations {
    private currentSessionId: string | null = null;
    private eventListeners: Map<string, UnlistenFn> = new Map();
    private agentEventCallbacks: Map<string, AgentEventCallback[]> = new Map();
    private fileEventCallbacks: FileEventCallback[] = [];

    async listModels(): Promise<LiteLLMModelsResponse> {
        // Use backend API via modelsAPI
        const modelsResponse = await modelsAPI.listModels();
        // Transform to LiteLLM format for compatibility
        return {
            object: "list",
            data: modelsResponse.models.map(model => ({
                id: model.id,
                object: "model",
                created: Math.floor(Date.now() / 1000),
                owned_by: model.description.split(" ")[0] || "unknown"
            }))
        };
    }

    async sendMessage(request: AgentChatRequest): Promise<AgentChatResponse> {
        try {
            const response = await invoke<AgentChatResponse>('chat_with_agent', {
                sessionId: request.session_id,
                message: request.message,
                model: request.model
            });
            
            // Store current session ID for event subscriptions
            this.currentSessionId = request.session_id;
            
            return response;
        } catch (error) {
            console.error('Failed to send message:', error);
            throw new Error(`Failed to send message: ${error}`);
        }
    }

    async subscribeToEvents(sessionId: string): Promise<void> {
        try {
            // Subscribe to backend events via Tauri command
            await invoke('subscribe_to_agent_events', { sessionId });
            
            // Set up Tauri event listener for this session if not already listening
            if (!this.eventListeners.has(sessionId)) {
                await this.setupEventListener(sessionId);
            }
        } catch (error) {
            console.error('Failed to subscribe to agent events:', error);
            throw new Error(`Failed to subscribe to events: ${error}`);
        }
    }

    async unsubscribeFromEvents(sessionId: string): Promise<void> {
        try {
            // Unsubscribe from backend events via Tauri command
            await invoke('unsubscribe_from_agent_events', { sessionId });
            
            // Clean up Tauri event listener
            await this.cleanupEventListener(sessionId);
        } catch (error) {
            console.error('Failed to unsubscribe from agent events:', error);
        }
    }

    async getSessionInfo(sessionId: string): Promise<AgentSessionInfo | null> {
        try {
            return await invoke<AgentSessionInfo | null>('get_agent_session_info', { sessionId });
        } catch (error) {
            console.error('Failed to get session info:', error);
            return null;
        }
    }

    async listSessions(): Promise<AgentSessionInfo[]> {
        try {
            return await invoke<AgentSessionInfo[]>('list_agent_sessions');
        } catch (error) {
            console.error('Failed to list sessions:', error);
            return [];
        }
    }

    async clearSession(sessionId: string): Promise<boolean> {
        try {
            return await invoke<boolean>('clear_agent_session', { sessionId });
        } catch (error) {
            console.error('Failed to clear session:', error);
            return false;
        }
    }

    async getAvailableTools(): Promise<AgentToolDefinition[]> {
        try {
            return await invoke<AgentToolDefinition[]>('get_available_agent_tools');
        } catch (error) {
            console.error('Failed to get available tools:', error);
            return [];
        }
    }

    async isAgentAvailable(): Promise<boolean> {
        try {
            return await invoke<boolean>('is_agent_available');
        } catch (error) {
            console.error('Failed to check agent availability:', error);
            return false;
        }
    }

    getCurrentSessionId(): string | null {
        return this.currentSessionId;
    }

    setCurrentSessionId(sessionId: string): void {
        this.currentSessionId = sessionId;
    }

    /**
     * Set up Tauri event listener for a specific session
     */
    private async setupEventListener(sessionId: string): Promise<void> {
        const eventName = `agent-event-${sessionId}`;
        
        try {
            const unlisten = await listen<AgentEvent>(eventName, (event) => {
                this.handleAgentEvent(sessionId, event.payload);
            });
            
            this.eventListeners.set(sessionId, unlisten);
            console.log(`Set up Tauri event listener for session: ${sessionId}`);
        } catch (error) {
            console.error(`Failed to set up event listener for session ${sessionId}:`, error);
            throw error;
        }
    }

    /**
     * Clean up Tauri event listener for a specific session
     */
    private async cleanupEventListener(sessionId: string): Promise<void> {
        const unlisten = this.eventListeners.get(sessionId);
        if (unlisten) {
            unlisten();
            this.eventListeners.delete(sessionId);
            
            // Also clean up callbacks for this session
            this.agentEventCallbacks.delete(sessionId);
            
            console.log(`Cleaned up event listener for session: ${sessionId}`);
        }
    }

    /**
     * Handle incoming agent events from Tauri
     */
    private handleAgentEvent(sessionId: string, event: AgentEvent): void {
        console.log(`Desktop agent event received for session ${sessionId}:`, event);
        
        // Call session-specific callbacks
        const sessionCallbacks = this.agentEventCallbacks.get(sessionId) || [];
        sessionCallbacks.forEach(callback => {
            try {
                callback(event);
            } catch (error) {
                console.error('Error in agent event callback:', error);
            }
        });
    }

    /**
     * Subscribe to agent events for a specific session
     * Returns unsubscribe function
     */
    onSessionEvents(sessionId: string, callback: AgentEventCallback): () => void {
        if (!this.agentEventCallbacks.has(sessionId)) {
            this.agentEventCallbacks.set(sessionId, []);
        }
        
        const callbacks = this.agentEventCallbacks.get(sessionId)!;
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
     * Subscribe to file events (global file system changes)
     * Returns unsubscribe function
     */
    onFileEvents(callback: FileEventCallback): () => void {
        this.fileEventCallbacks.push(callback);
        
        // Set up global file event listener if not already done
        if (!this.eventListeners.has('__file_events__')) {
            this.setupFileEventListener();
        }
        
        // Return unsubscribe function
        return () => {
            const index = this.fileEventCallbacks.indexOf(callback);
            if (index > -1) {
                this.fileEventCallbacks.splice(index, 1);
            }
        };
    }

    /**
     * Set up global file event listener
     */
    private async setupFileEventListener(): Promise<void> {
        try {
            const unlisten = await listen<{event_type: string, event_data: any}>('file-event', (event) => {
                this.handleFileEvent(event.payload.event_type, event.payload.event_data);
            });
            
            this.eventListeners.set('__file_events__', unlisten);
            console.log('Set up global file event listener');
        } catch (error) {
            console.error('Failed to set up file event listener:', error);
        }
    }

    /**
     * Handle incoming file events from file watcher
     */
    private handleFileEvent(type: string, event: any): void {
        console.log(`Desktop file event received (${type}):`, event);
        
        // Call all file event callbacks
        this.fileEventCallbacks.forEach(callback => {
            try {
                callback(type, event);
            } catch (error) {
                console.error('Error in file event callback:', error);
            }
        });
    }

    /**
     * Clean up all event listeners
     */
    async destroy(): Promise<void> {
        // Clean up all session event listeners
        for (const [sessionId] of this.eventListeners) {
            if (sessionId !== '__file_events__') {
                await this.cleanupEventListener(sessionId);
            }
        }
        
        // Clean up file event listener
        const fileEventUnlisten = this.eventListeners.get('__file_events__');
        if (fileEventUnlisten) {
            fileEventUnlisten();
            this.eventListeners.delete('__file_events__');
        }
        
        // Clear callbacks
        this.agentEventCallbacks.clear();
        this.fileEventCallbacks.length = 0;
        
        console.log('Desktop agent adapter destroyed');
    }
}