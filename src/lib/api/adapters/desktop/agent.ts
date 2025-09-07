// Desktop Agent Operations via Tauri Commands
import { invoke } from '@tauri-apps/api/core';
import type { 
    AgentOperations, 
    AgentChatRequest, 
    AgentChatResponse, 
    AgentSessionInfo,
    AgentToolDefinition,
    LiteLLMModelsResponse 
} from '../../types';

export class DesktopAgentAdapter implements AgentOperations {
    private currentSessionId: string | null = null;

    async listModels(): Promise<LiteLLMModelsResponse> {
        try {
            // For desktop, we need to make HTTP request directly to LiteLLM
            const liteLLMUrl = import.meta.env.VITE_LITELLM_BASE_URL || 'http://localhost:4000';
            const response = await fetch(`${liteLLMUrl}/v1/models`, {
                headers: {
                    'Authorization': `Bearer ${import.meta.env.VITE_LITELLM_API_KEY || 'your-api-key'}`
                }
            });
            
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }
            
            return await response.json();
        } catch (error) {
            console.error('Failed to fetch models:', error);
            throw new Error(`Failed to fetch models: ${error}`);
        }
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
            await invoke('subscribe_to_agent_events', { sessionId });
        } catch (error) {
            console.error('Failed to subscribe to agent events:', error);
            throw new Error(`Failed to subscribe to events: ${error}`);
        }
    }

    async unsubscribeFromEvents(sessionId: string): Promise<void> {
        try {
            await invoke('unsubscribe_from_agent_events', { sessionId });
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
}