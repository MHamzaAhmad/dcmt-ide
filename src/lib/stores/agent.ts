import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { agentAPI } from '$lib/api/agent';
import { eventStore } from './events';
import type { QueryClient } from '@tanstack/svelte-query';
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
    currentToolStatus: { toolName: string; status: string } | null;
    
    // UI State
    isProcessing: boolean;
    streamingContent: string;
    streamingMessageId: string | null;
    currentJobId: string | null;
    
    // Events
    recentEvents: AgentEvent[];
    maxRecentEvents: number;
    processedEvents: Map<string, number>;
    eventDeduplicationWindow: number;
    
    // Agent execution state
    isAgentRunning: boolean;
    modifiedFiles: Map<string, any>;
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
        currentToolStatus: null,
        
        isProcessing: false,
        streamingContent: '',
        streamingMessageId: null,
        currentJobId: null,
        
        recentEvents: [],
        maxRecentEvents: 50,
        processedEvents: new Map(),
        eventDeduplicationWindow: 5000, // 5 seconds
        
        isAgentRunning: false,
        modifiedFiles: new Map()
    };

    const { subscribe, set, update } = writable<AgentState>(initialState);

    // Unified event and file sync management
    let isInitialized = false;
    let queryClient: QueryClient | undefined;

    const store = {
        subscribe,
        set,
        update,

        // Set the query client (should be called from component context)
        setQueryClient(client: QueryClient): void {
            queryClient = client;
        },

        // Initialization
        async initialize(): Promise<void> {
            if (!browser || isInitialized) return;

            try {
                // Check availability
                const isAvailable = await agentAPI.isAgentAvailable();
                
                // Load models
                store.loadModels();
                
                // Load available tools
                const tools = await agentAPI.getAvailableTools();
                
                // Agent completion is handled directly by the WebSocket event handler
                // This ensures immediate coordination without circular dependencies
                
                update(state => ({
                    ...state,
                    isAvailable,
                    isConnected: true,
                    connectionError: null,
                    availableTools: tools
                }));

                isInitialized = true;
                console.log('Agent store initialized successfully');

            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : 'Failed to initialize agent';
                console.error('Agent store initialization failed:', errorMessage);
                
                update(state => ({
                    ...state,
                    isAvailable: false,
                    isConnected: false,
                    connectionError: errorMessage
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
            
            // Set up platform-appropriate event subscription
            console.log('Subscribing to events for session:', sessionId);
            
            // Use platform-specific event handling
            const adapter = await agentAPI.getAdapter();
            
            // Desktop uses Tauri events, web uses SSE
            if ('onSessionEvents' in adapter) {
                // Desktop (Tauri) - use adapter's event system
                const unsubscribe = (adapter as any).onSessionEvents(sessionId, store.handleAgentEvent);
            } else {
                // Web - use SSE adapter for agent events
                const sseAdapter = await agentAPI.getSSEAdapter();
                if (sseAdapter) {
                    const agentUnsubscribe = sseAdapter.onSession(sessionId, store.handleAgentEvent);
                }
            }
            
            // Subscribe to backend agent events
            await agentAPI.subscribeToEvents(sessionId);
            await agentAPI.setCurrentSessionId(sessionId);
            
            // Store cleanup functions for session management
            
            update(state => ({
                ...state,
                currentSessionId: sessionId,
                messages: [],
                fileSyncActive: true,
                connectionError: null
            }));

            console.log(`Created agent session with unified event handling: ${sessionId}`);
            return sessionId;
        },

        async clearSession(): Promise<void> {
            const state = store.getCurrentState();
            
            if (state.currentSessionId) {
                await agentAPI.clearSession(state.currentSessionId);
                await agentAPI.unsubscribeFromEvents(state.currentSessionId);
            }

            // Event handlers cleaned up automatically with session unsubscribe

            update(state => ({
                ...state,
                currentSessionId: null,
                sessionInfo: null,
                messages: [],
                activeToolResults: new Map(),
                isProcessing: false,
                streamingContent: '',
                streamingMessageId: null,
                currentJobId: null,
                fileSyncActive: false,
                fileOperationsCount: 0,
                modifiedFiles: new Map(),
                isAgentRunning: false
            }));
            
            console.log('Cleared agent session and cleaned up event handlers');
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

        // Event handling (now used by unified hooks)
        handleAgentEvent(event: AgentEvent): void {
            console.log('Agent event received via unified hooks:', event);
            console.log('Event type:', event.type);
            if (event.type === 'LLMStreaming') {
                console.log('LLMStreaming event content:', event.content);
                console.log('Full event object:', JSON.stringify(event, null, 2));
            }

            // Check for event deduplication using flattened metadata fields
            if (event.event_id) {
                const currentState = store.getCurrentState();
                const now = Date.now();
                
                // Check if we've already processed this event
                if (currentState.processedEvents.has(event.event_id)) {
                    console.log(`AgentStore: Skipping duplicate event ${event.event_id}`);
                    return;
                }
                
                // Clean up old events outside the deduplication window
                const cutoffTime = now - currentState.eventDeduplicationWindow;
                const newProcessedEvents = new Map();
                for (const [eventId, timestamp] of currentState.processedEvents) {
                    if (timestamp > cutoffTime) {
                        newProcessedEvents.set(eventId, timestamp);
                    }
                }
                newProcessedEvents.set(event.event_id, now);
                
                // Update processed events
                update(state => ({
                    ...state,
                    processedEvents: newProcessedEvents
                }));
            }

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
                        currentJobId: event.job_id,
                        modifiedFiles: new Map(), // Clear previous run's files
                        isAgentRunning: true
                    }));
                    console.log('AgentStore: Agent run started, file systems pausing');
                    break;

                case 'LLMCallStart':
                    console.log('Processing LLMCallStart in agent store');
                    // Reset streaming state for new LLM call
                    update(state => {
                        const streamingMessage: AgentChatMessage = {
                            id: `msg-streaming-${Date.now()}`,
                            role: 'assistant',
                            content: '',
                            timestamp: new Date(),
                            status: 'streaming',
                            streaming: true,
                            model: state.selectedModel?.id
                        };
                        
                        console.log('Creating initial streaming message for LLMCallStart');
                        
                        return {
                            ...state,
                            messages: [...state.messages, streamingMessage],
                            streamingContent: '',
                            streamingMessageId: streamingMessage.id
                        };
                    });
                    break;

                case 'StreamChunk':
                    // Process streaming chunk - create message if needed, then append content
                    console.log('Processing StreamChunk in agent store, content:', event.content);
                    update(state => {
                        if (!state.streamingMessageId) {
                            // Create a new streaming message if we don't have one
                            const streamingMessage: AgentChatMessage = {
                                id: `msg-streaming-${Date.now()}`,
                                role: 'assistant',
                                content: event.content || '',
                                timestamp: new Date(),
                                status: 'streaming',
                                streaming: true,
                                model: state.selectedModel?.id
                            };
                            
                            console.log('Creating new streaming message:', streamingMessage);
                            
                            return {
                                ...state,
                                messages: [...state.messages, streamingMessage],
                                streamingContent: event.content || '',
                                streamingMessageId: streamingMessage.id
                            };
                        } else {
                            // Update existing streaming message
                            const newContent = state.streamingContent + (event.content || '');
                            console.log('Updating streaming message, new content length:', newContent.length);
                            
                            return {
                                ...state,
                                messages: state.messages.map(msg => 
                                    msg.id === state.streamingMessageId 
                                        ? { ...msg, content: newContent }
                                        : msg
                                ),
                                streamingContent: newContent
                            };
                        }
                    });
                    break;

                case 'ToolCallStart':
                    // Tool call starting - show preparation status
                    console.log('Tool call starting:', event.tool_name);
                    update(state => ({
                        ...state,
                        currentToolStatus: {
                            toolName: event.tool_name,
                            status: 'preparing'
                        }
                    }));
                    break;

                case 'ToolCallReady':
                    // Tool call arguments complete and ready for execution
                    console.log('Tool call ready for execution:', event.tool_call);
                    // Tool execution will be handled by backend automatically
                    break;

                case 'ToolExecuting':
                    // Update tool status for floating badge
                    update(state => ({
                        ...state,
                        currentToolStatus: {
                            toolName: event.tool,
                            status: 'executing'
                        }
                    }));
                    
                    store.updateToolResult(`temp-${event.tool}`, {
                        tool_call_id: `temp-${event.tool}`,
                        tool_name: event.tool,
                        status: 'executing',
                        started_at: new Date()
                    });
                    break;

                case 'ToolCompleted':
                    // Clear tool status when completed
                    update(state => ({
                        ...state,
                        currentToolStatus: null
                    }));
                    
                    store.updateToolResult(`temp-${event.tool}`, {
                        tool_call_id: `temp-${event.tool}`,
                        tool_name: event.tool,
                        status: 'completed',
                        result: event.result,
                        completed_at: new Date()
                    });
                    
                    // THIS IS THE SINGLE PLACE WHERE AGENT EVENTS BECOME FILE EVENTS
                    // Use flattened metadata fields directly on event object
                    if (event.is_file_operation && event.file_paths && event.file_paths.length > 0) {
                        console.log(`AgentStore: Processing file operation ${event.tool} on ${event.file_paths.join(', ')}`);
                        
                        // Skip read operations - they don't modify files
                        if (event.tool === 'read_file') {
                            console.log(`AgentStore: Skipping read operation for ${event.file_paths.join(', ')}`);
                            break;
                        }
                        
                        // Convert to appropriate file event for each path
                        for (const path of event.file_paths) {
                            let changeType: 'created' | 'modified' | 'deleted';
                            switch (event.tool) {
                                case 'create_file':
                                case 'write_file': // write_file creates if doesn't exist
                                    changeType = 'created';
                                    break;
                                case 'delete_file':
                                    changeType = 'deleted';
                                    break;
                                case 'update_file':
                                    changeType = 'modified';
                                    break;
                                default:
                                    // Only count actual modification tools, not read operations
                                    console.log(`AgentStore: Unknown file operation tool: ${event.tool}, treating as modified`);
                                    changeType = 'modified';
                                    break;
                            }

                            // Track modified files during agent run instead of emitting events immediately
                            const currentState = store.getCurrentState();
                            if (currentState.isAgentRunning) {
                                update(state => {
                                    const newModifiedFiles = new Map(state.modifiedFiles);
                                    newModifiedFiles.set(path, {
                                        tool: event.tool,
                                        changeType,
                                        timestamp: Date.now()
                                    });
                                    return {
                                        ...state,
                                        modifiedFiles: newModifiedFiles
                                    };
                                });
                            } else {
                                // Emit file system event immediately if not in agent run
                                eventStore.events.fileModified(path, undefined, 'agent');
                            }
                        }
                    } else {
                        // Non-file tool completed
                        console.log(`AgentStore: Non-file tool completed: ${event.tool}`);
                    }
                    break;

                case 'LLMCallComplete':
                    console.log('Processing LLMCallComplete in agent store');
                    // Finalize the streaming message
                    update(state => {
                        if (state.streamingMessageId) {
                            console.log('Finalizing streaming message:', state.streamingMessageId);
                            return {
                                ...state,
                                messages: state.messages.map(msg => 
                                    msg.id === state.streamingMessageId 
                                        ? { ...msg, status: 'completed', streaming: false }
                                        : msg
                                ),
                                streamingContent: '',
                                streamingMessageId: null
                            };
                        }
                        console.log('No streaming message to finalize');
                        return state;
                    });
                    break;

                case 'JobComplete':
                    // Handle UI state updates only - file system coordination is handled via EventStore
                    store.handleJobCompletionUI(event).catch(error => {
                        console.error('Error handling job completion UI:', error);
                    });
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

        // Handle job completion - simplified
        async handleJobCompletionUI(event: any): Promise<void> {
            console.log('AgentStore: Handling agent run completion');
            
            update(state => ({
                ...state,
                isProcessing: false,
                streamingContent: '',
                streamingMessageId: null,
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
        },

        // Session switching
        async switchSession(newSessionId: string): Promise<void> {
            const currentState = store.getCurrentState();
            
            if (currentState.currentSessionId === newSessionId) {
                return; // Already on this session
            }
            
            // Subscribe to new session
            await agentAPI.subscribeToEvents(newSessionId);
            await agentAPI.setCurrentSessionId(newSessionId);
            
            update(state => ({
                ...state,
                currentSessionId: newSessionId,
                connectionError: null
            }));
            
            console.log(`Switched to agent session: ${newSessionId}`);
        },

        // Get sync status
        getSyncStatus() {
            const currentState = store.getCurrentState();
            return {
                eventsActive: currentState.currentSessionId !== null,
                currentSession: currentState.currentSessionId
            };
        },


        // Cleanup
        async destroy(): Promise<void> {
            // Event handlers cleaned up automatically with session unsubscribe
            
            isInitialized = false;
            console.log('Agent store destroyed and cleaned up');
        }
    };

    return store;
}

export const agentStore = createAgentStore();