// Unified Agent API - Platform-agnostic agent operations
import { browser } from '$app/environment';
import { isDesktop } from '$lib/utils/platform';
import type { 
    AgentOperations, 
    AgentChatRequest, 
    AgentChatResponse, 
    AgentSessionInfo,
    AgentToolDefinition,
    LiteLLMModelsResponse 
} from './types';

let agentAdapter: AgentOperations | null = null;

// Initialize the appropriate adapter based on platform
async function initializeAdapter(): Promise<AgentOperations> {
    if (agentAdapter) return agentAdapter;
    
    if (!browser) {
        throw new Error('Agent operations are only available in browser environment');
    }

    if (isDesktop()) {
        const { DesktopAgentAdapter } = await import('./adapters/desktop/agent');
        agentAdapter = new DesktopAgentAdapter();
    } else {
        const { WebAgentAdapter } = await import('./adapters/web/agent');
        agentAdapter = new WebAgentAdapter();
    }

    return agentAdapter;
}

export const agentAPI = {
    /**
     * Get the current agent adapter (initialize if needed)
     */
    async getAdapter(): Promise<AgentOperations> {
        return await initializeAdapter();
    },

    /**
     * Check if agent is available (has project workspace)
     */
    async isAgentAvailable(): Promise<boolean> {
        try {
            const adapter = await initializeAdapter();
            return await adapter.isAgentAvailable();
        } catch (error) {
            console.error('Failed to check agent availability:', error);
            return false;
        }
    },

    /**
     * List all available models from LiteLLM
     */
    async listModels(): Promise<LiteLLMModelsResponse> {
        const adapter = await initializeAdapter();
        return await adapter.listModels();
    },

    /**
     * Send a message to the agent
     */
    async sendMessage(request: AgentChatRequest): Promise<AgentChatResponse> {
        const adapter = await initializeAdapter();
        return await adapter.sendMessage(request);
    },

    /**
     * Subscribe to agent events for a session
     */
    async subscribeToEvents(sessionId: string): Promise<void> {
        const adapter = await initializeAdapter();
        return await adapter.subscribeToEvents(sessionId);
    },

    /**
     * Unsubscribe from agent events for a session
     */
    async unsubscribeFromEvents(sessionId: string): Promise<void> {
        const adapter = await initializeAdapter();
        return await adapter.unsubscribeFromEvents(sessionId);
    },

    /**
     * Get information about a specific session
     */
    async getSessionInfo(sessionId: string): Promise<AgentSessionInfo | null> {
        const adapter = await initializeAdapter();
        return await adapter.getSessionInfo(sessionId);
    },

    /**
     * List all active sessions
     */
    async listSessions(): Promise<AgentSessionInfo[]> {
        const adapter = await initializeAdapter();
        return await adapter.listSessions();
    },

    /**
     * Clear/remove a specific session
     */
    async clearSession(sessionId: string): Promise<boolean> {
        const adapter = await initializeAdapter();
        return await adapter.clearSession(sessionId);
    },

    /**
     * Get available tool definitions
     */
    async getAvailableTools(): Promise<AgentToolDefinition[]> {
        const adapter = await initializeAdapter();
        return await adapter.getAvailableTools();
    },

    /**
     * Get current session ID
     */
    async getCurrentSessionId(): Promise<string | null> {
        const adapter = await initializeAdapter();
        return adapter.getCurrentSessionId();
    },

    /**
     * Set current session ID
     */
    async setCurrentSessionId(sessionId: string): Promise<void> {
        const adapter = await initializeAdapter();
        adapter.setCurrentSessionId(sessionId);
    },

    /**
     * Generate a new session ID
     */
    generateSessionId(): string {
        return `session-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    },

    /**
     * Get platform-specific WebSocket adapter (for web platform)
     */
    async getWebSocketAdapter(): Promise<any> {
        if (isDesktop()) {
            return null; // Desktop uses Tauri event system
        }
        
        const { agentWebSocket } = await import('./adapters/web/agentWebSocket');
        return agentWebSocket;
    }
};

// Export the adapter instance for direct access if needed
export { agentAdapter };