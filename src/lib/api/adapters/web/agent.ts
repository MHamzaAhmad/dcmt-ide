// Web Agent Operations via HTTP API
import { apiClient } from '../../client';
import { modelsAPI } from '../../models';
import type {
    AgentOperations,
    AgentChatRequest,
    AgentChatResponse,
    AgentSessionInfo,
    AgentToolDefinition,
    LiteLLMModelsResponse
} from '../../types';

export class WebAgentAdapter implements AgentOperations {
    private currentSessionId: string | null = null;

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
            // Use the backend /api/agent/chat endpoint
            const response = await apiClient.post<AgentChatResponse>('/api/agent/chat', request);
            
            // Store current session ID for consistency
            this.currentSessionId = request.session_id;
            
            return response;
        } catch (error) {
            console.error('Failed to send message via backend:', error);
            throw new Error(`Failed to send message: ${error}`);
        }
    }

    async subscribeToEvents(sessionId: string): Promise<void> {
        // For web, event subscription is handled by WebSocket adapter
        // This is a no-op as WebSocket handles subscriptions automatically
        console.log(`Subscribing to events for session: ${sessionId}`);
    }

    async unsubscribeFromEvents(sessionId: string): Promise<void> {
        // For web, event unsubscription is handled by WebSocket adapter
        console.log(`Unsubscribing from events for session: ${sessionId}`);
    }

    async getSessionInfo(sessionId: string): Promise<AgentSessionInfo | null> {
        // Mock session info for now
        return {
            id: sessionId,
            created_at: new Date().toISOString(),
            last_activity: new Date().toISOString(),
            message_count: 0
        };
    }

    async listSessions(): Promise<AgentSessionInfo[]> {
        // Mock empty sessions list for now
        return [];
    }

    async clearSession(sessionId: string): Promise<boolean> {
        // Mock session clear for now
        return true;
    }

    async getAvailableTools(): Promise<AgentToolDefinition[]> {
        // Mock tools list for now
        return [];
    }

    async isAgentAvailable(): Promise<boolean> {
        // For web, agent is always available (no project selection needed)
        return true;
    }

    getCurrentSessionId(): string | null {
        return this.currentSessionId;
    }

    setCurrentSessionId(sessionId: string): void {
        this.currentSessionId = sessionId;
    }
}