import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { isDesktop } from '$lib/utils/platform';
import { agentAPI } from '$lib/api/agent';
import type { 
    AgentEvent, 
    AgentChatMessage, 
    AgentChatRequest,
    AgentSessionInfo, 
    AgentToolDefinition,
    AgentToolResult,
    LiteLLMModel
} from '$lib/api/types';

export interface AgentState {
    // Connection & Availability
    isAvailable: boolean;
    isConnected: boolean;
    connectionError: string | null;
    
    // Models
    availableModels: LiteLLMModel[];
    selectedModel: LiteLLMModel | null;
    isLoadingModels: boolean;
    modelsError: string | null;
    
    // Current Session
    currentSessionId: string | null;
    sessionInfo: AgentSessionInfo | null;
    
    // Messages & Tools
    messages: AgentChatMessage[];
    activeToolResults: Map<string, AgentToolResult>;
    availableTools: AgentToolDefinition[];
    
    // UI State
    isProcessing: boolean;
    streamingContent: string;
    currentJobId: string | null;
    
    // Events
    recentEvents: AgentEvent[];
    maxRecentEvents: number;
}

function createAgentStore() {
    const initialState: AgentState = {
        isAvailable: false,
        isConnected: false,
        connectionError: null,
        
        availableModels: [],
        selectedModel: null,
        isLoadingModels: false,
        modelsError: null,
        
        currentSessionId: null,
        sessionInfo: null,
        
        messages: [],
        activeToolResults: new Map(),
        availableTools: [],
        
        isProcessing: false,
        streamingContent: '',
        currentJobId: null,
        
        recentEvents: [],
        maxRecentEvents: 50
    };

    const { subscribe, set, update } = writable<AgentState>(initialState);

    // Event handlers
    let eventUnsubscriber: (() => void) | null = null;
    let sessionUnsubscriber: (() => void) | null = null;

    const store = {
        subscribe,
        set,
        update,

        // Initialization
        async initialize(): Promise<void> {
            if (!browser) return;

            try {
                // Check availability
                const isAvailable = await agentAPI.isAgentAvailable();
                
                // Load models
                store.loadModels();
                
                // Load available tools
                const tools = await agentAPI.getAvailableTools();
                
                update(state => ({
                    ...state,
                    isAvailable,
                    isConnected: true,
                    connectionError: null,
                    availableTools: tools
                }));

                // Set up event listeners for web platform
                if (!isDesktop()) {
                    const agentWebSocket = await agentAPI.getWebSocketAdapter();
                    if (agentWebSocket) {
                        eventUnsubscriber = agentWebSocket.onAny((event: AgentEvent) => {
                            store.handleAgentEvent(event);
                        });
                    }
                }

            } catch (error) {
                update(state => ({
                    ...state,
                    isAvailable: false,
                    isConnected: false,
                    connectionError: error instanceof Error ? error.message : 'Failed to initialize agent'
                }));
            }
        },

        // Models management
        async loadModels(): Promise<void> {
            update(state => ({
                ...state,
                isLoadingModels: true,
                modelsError: null
            }));

            try {
                const response = await agentAPI.listModels();
                
                update(state => ({
                    ...state,
                    availableModels: response.data,
                    isLoadingModels: false,
                    selectedModel: state.selectedModel || response.data[0] || null
                }));
            } catch (error) {
                update(state => ({
                    ...state,
                    isLoadingModels: false,
                    modelsError: error instanceof Error ? error.message : 'Failed to load models'
                }));
            }
        },

        setSelectedModel(model: LiteLLMModel | null): void {
            update(state => ({
                ...state,
                selectedModel: model
            }));
        },

        // Session management
        async createSession(): Promise<string> {
            const sessionId = agentAPI.generateSessionId();
            
            update(state => ({
                ...state,
                currentSessionId: sessionId,
                messages: []
            }));

            // Subscribe to session events
            await agentAPI.subscribeToEvents(sessionId);
            await agentAPI.setCurrentSessionId(sessionId);

            // For web platform, subscribe to WebSocket events
            if (!isDesktop()) {
                const agentWebSocket = await agentAPI.getWebSocketAdapter();
                if (agentWebSocket) {
                    sessionUnsubscriber = agentWebSocket.onSession(sessionId, (event: AgentEvent) => {
                        store.handleAgentEvent(event);
                    });
                }
            }

            return sessionId;
        },

        async clearSession(): Promise<void> {
            const state = store.getCurrentState();
            
            if (state.currentSessionId) {
                await agentAPI.clearSession(state.currentSessionId);
                await agentAPI.unsubscribeFromEvents(state.currentSessionId);
            }

            if (sessionUnsubscriber) {
                sessionUnsubscriber();
                sessionUnsubscriber = null;
            }

            update(state => ({
                ...state,
                currentSessionId: null,
                sessionInfo: null,
                messages: [],
                activeToolResults: new Map(),
                isProcessing: false,
                streamingContent: '',
                currentJobId: null
            }));
        },

        // Message handling
        async sendMessage(content: string): Promise<void> {
            const state = store.getCurrentState();
            if (!state.selectedModel) throw new Error('No model selected');
            if (!state.currentSessionId) {
                await store.createSession();
            }

            const currentState = store.getCurrentState();
            const sessionId = currentState.currentSessionId!;

            // Add user message
            const userMessage: AgentChatMessage = {
                id: `msg-${Date.now()}`,
                role: 'user',
                content,
                timestamp: new Date(),
                status: 'sending'
            };

            store.addMessage(userMessage);

            try {
                // Update message status
                store.updateMessageStatus(userMessage.id, 'completed');

                // Send to agent
                const request: AgentChatRequest = {
                    session_id: sessionId,
                    message: content,
                    model: state.selectedModel.id
                };

                const response = await agentAPI.sendMessage(request);

                update(state => ({
                    ...state,
                    isProcessing: true,
                    currentJobId: response.job_id
                }));

            } catch (error) {
                store.updateMessageStatus(userMessage.id, 'error', 
                    error instanceof Error ? error.message : 'Failed to send message');
                
                update(state => ({
                    ...state,
                    isProcessing: false
                }));
            }
        },

        addMessage(message: AgentChatMessage): void {
            update(state => ({
                ...state,
                messages: [...state.messages, message]
            }));
        },

        updateMessageStatus(messageId: string, status: AgentChatMessage['status'], error?: string): void {
            update(state => ({
                ...state,
                messages: state.messages.map(msg => 
                    msg.id === messageId 
                        ? { ...msg, status, error }
                        : msg
                )
            }));
        },

        // Tool execution tracking
        updateToolResult(toolCallId: string, result: Partial<AgentToolResult>): void {
            update(state => {
                const newResults = new Map(state.activeToolResults);
                const existing = newResults.get(toolCallId);
                newResults.set(toolCallId, { ...existing, ...result } as AgentToolResult);
                
                return {
                    ...state,
                    activeToolResults: newResults
                };
            });
        },

        clearToolResult(toolCallId: string): void {
            update(state => {
                const newResults = new Map(state.activeToolResults);
                newResults.delete(toolCallId);
                
                return {
                    ...state,
                    activeToolResults: newResults
                };
            });
        },

        // Event handling
        handleAgentEvent(event: AgentEvent): void {
            console.log('Agent event received:', event);

            // Add to recent events
            update(state => ({
                ...state,
                recentEvents: [...state.recentEvents.slice(-state.maxRecentEvents + 1), event]
            }));

            switch (event.type) {
                case 'JobQueued':
                    update(state => ({
                        ...state,
                        isProcessing: true,
                        currentJobId: event.job_id
                    }));
                    break;

                case 'LLMCallStart':
                    update(state => ({
                        ...state,
                        streamingContent: ''
                    }));
                    break;

                case 'LLMStreaming':
                    update(state => ({
                        ...state,
                        streamingContent: state.streamingContent + event.content
                    }));
                    break;

                case 'ToolExecuting':
                    store.updateToolResult(`temp-${event.tool}`, {
                        tool_call_id: `temp-${event.tool}`,
                        tool_name: event.tool,
                        status: 'executing',
                        started_at: new Date()
                    });
                    break;

                case 'ToolCompleted':
                    store.updateToolResult(`temp-${event.tool}`, {
                        tool_call_id: `temp-${event.tool}`,
                        tool_name: event.tool,
                        status: 'completed',
                        result: event.result,
                        completed_at: new Date()
                    });
                    break;

                case 'JobComplete':
                    // Add assistant message
                    const assistantMessage: AgentChatMessage = {
                        id: `msg-${Date.now()}`,
                        role: 'assistant',
                        content: event.response,
                        timestamp: new Date(),
                        status: 'completed',
                        model: store.getCurrentState().selectedModel?.id
                    };

                    store.addMessage(assistantMessage);

                    update(state => ({
                        ...state,
                        isProcessing: false,
                        streamingContent: '',
                        currentJobId: null
                    }));

                    // Clear completed tool results after a delay
                    setTimeout(() => {
                        const state = store.getCurrentState();
                        state.activeToolResults.forEach((result, id) => {
                            if (result.status === 'completed') {
                                store.clearToolResult(id);
                            }
                        });
                    }, 3000);
                    break;

                case 'Error':
                    update(state => ({
                        ...state,
                        isProcessing: false,
                        connectionError: event.message,
                        currentJobId: null
                    }));
                    break;
            }
        },

        // Utility
        getCurrentState(): AgentState {
            let currentState: AgentState;
            update(state => {
                currentState = state;
                return state;
            });
            return currentState!;
        },

        // Cleanup
        async destroy(): Promise<void> {
            if (eventUnsubscriber) {
                eventUnsubscriber();
                eventUnsubscriber = null;
            }
            
            if (sessionUnsubscriber) {
                sessionUnsubscriber();
                sessionUnsubscriber = null;
            }

            if (!isDesktop()) {
                const agentWebSocket = await agentAPI.getWebSocketAdapter();
                if (agentWebSocket) {
                    agentWebSocket.destroy();
                }
            }
        }
    };

    return store;
}

export const agentStore = createAgentStore();