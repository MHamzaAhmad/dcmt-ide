import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { agentAPI } from '$lib/api/agent';
import { useAgentEvents, useAgentFileSync } from '$lib/api/hooks';
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
    
    // UI State
    isProcessing: boolean;
    streamingContent: string;
    streamingMessageId: string | null;
    currentJobId: string | null;
    
    // Events & File Sync
    recentEvents: AgentEvent[];
    maxRecentEvents: number;
    fileSyncActive: boolean;
    fileOperationsCount: number;
    
    // Agent run coordination
    modifiedFiles: Map<string, {
        tool: string;
        changeType: 'created' | 'modified' | 'deleted';
        timestamp: number;
    }>;
    isAgentRunning: boolean;
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
        streamingMessageId: null,
        currentJobId: null,
        
        recentEvents: [],
        maxRecentEvents: 50,
        fileSyncActive: false,
        fileOperationsCount: 0,
        
        modifiedFiles: new Map(),
        isAgentRunning: false
    };

    const { subscribe, set, update } = writable<AgentState>(initialState);

    // Unified event and file sync management
    let agentEvents: ReturnType<typeof useAgentEvents> | null = null;
    let agentFileSync: ReturnType<typeof useAgentFileSync> | null = null;
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
            
            // Set up unified event handling for this session
            agentEvents = useAgentEvents({
                sessionId,
                onAgentEvent: store.handleAgentEvent,
                onFileEvent: (type, event) => {
                    console.log(`File event from agent session: ${type}`, event);
                },
                enabled: true,
                queryClient
            });

            // Set up file synchronization
            agentFileSync = useAgentFileSync({
                sessionId,
                enabled: true,
                onFileChanged: (path, changeType) => {
                    console.log(`Agent modified file: ${path} (${changeType})`);
                    // Integrate with new store system
                    store.handleAgentFileChange(path, changeType);
                },
                onAgentFileOperation: (tool, path, result) => {
                    update(state => ({
                        ...state,
                        fileOperationsCount: state.fileOperationsCount + 1
                    }));
                    // Integrate with new store system
                    store.handleAgentToolOperation(tool, path, result);
                },
                queryClient
            });

            // Subscribe to backend agent events
            await agentAPI.subscribeToEvents(sessionId);
            await agentAPI.setCurrentSessionId(sessionId);
            
            // Start event handlers
            await agentEvents.start(sessionId);
            await agentFileSync.start();
            
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

            // Clean up unified event handlers
            if (agentEvents) {
                agentEvents.destroy();
                agentEvents = null;
            }

            if (agentFileSync) {
                agentFileSync.destroy();
                agentFileSync = null;
            }

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
                    console.log('AgentStore: Agent run started, tracking file modifications');
                    break;

                case 'LLMCallStart':
                    // Create a streaming message when LLM starts
                    const streamingMessage: AgentChatMessage = {
                        id: `msg-streaming-${Date.now()}`,
                        role: 'assistant',
                        content: '',
                        timestamp: new Date(),
                        status: 'streaming',
                        streaming: true,
                        model: store.getCurrentState().selectedModel?.id
                    };
                    
                    store.addMessage(streamingMessage);
                    
                    update(state => ({
                        ...state,
                        streamingContent: '',
                        streamingMessageId: streamingMessage.id
                    }));
                    break;

                case 'LLMStreaming':
                    update(state => {
                        const newContent = state.streamingContent + event.content;
                        
                        // Update the streaming message with new content
                        if (state.streamingMessageId) {
                            const messages = state.messages.map(msg => 
                                msg.id === state.streamingMessageId
                                    ? { ...msg, content: newContent }
                                    : msg
                            );
                            
                            return {
                                ...state,
                                streamingContent: newContent,
                                messages
                            };
                        }
                        
                        return {
                            ...state,
                            streamingContent: newContent
                        };
                    });
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
                    
                    // File operations are handled by agentFileSync
                    console.log(`Tool completed: ${event.tool}`);
                    break;

                case 'JobComplete':
                    // Handle agent run completion with file coordination
                    // Handle completion asynchronously without blocking the event handler
                    store.handleJobCompletion(event).catch(error => {
                        console.error('Error handling job completion:', error);
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

        // Handle job completion with coordinated file refresh and compilation
        async handleJobCompletion(event: any): Promise<void> {
            console.log('AgentStore: Handling agent run completion');
            
            const currentState = store.getCurrentState();
            const currentStreamingContent = currentState.streamingContent;
            let finalContent = currentStreamingContent || '';
            
            // If no streaming content, try to parse the response
            if (!finalContent && event.response) {
                try {
                    const parsed = JSON.parse(event.response);
                    finalContent = parsed.message || parsed.content || parsed.response || event.response;
                } catch {
                    finalContent = event.response;
                }
            }
            
            // Update UI state first
            if (currentState.streamingMessageId) {
                update(state => ({
                    ...state,
                    messages: state.messages.map(msg => 
                        msg.id === state.streamingMessageId
                            ? { ...msg, content: finalContent, status: 'completed', streaming: false }
                            : msg
                    ),
                    isProcessing: false,
                    streamingContent: '',
                    streamingMessageId: null,
                    currentJobId: null,
                    isAgentRunning: false
                }));
            } else {
                const assistantMessage: AgentChatMessage = {
                    id: `msg-${Date.now()}`,
                    role: 'assistant',
                    content: finalContent,
                    timestamp: new Date(),
                    status: 'completed',
                    model: currentState.selectedModel?.id
                };

                store.addMessage(assistantMessage);
                update(state => ({
                    ...state,
                    isProcessing: false,
                    streamingContent: '',
                    streamingMessageId: null,
                    currentJobId: null,
                    isAgentRunning: false
                }));
            }
            
            // Now handle file coordination
            await store.coordinateFileRefreshAndCompilation();
            
            // Clear completed tool results after coordination
            setTimeout(() => {
                const state = store.getCurrentState();
                state.activeToolResults.forEach((result, id) => {
                    if (result.status === 'completed') {
                        store.clearToolResult(id);
                    }
                });
            }, 3000);
        },

        // Coordinate file refresh and compilation after agent run
        async coordinateFileRefreshAndCompilation(): Promise<void> {
            const currentState = store.getCurrentState();
            const modifiedFiles = currentState.modifiedFiles;
            
            if (modifiedFiles.size === 0) {
                console.log('AgentStore: No files were modified during agent run');
                return;
            }
            
            console.log(`AgentStore: Coordinating refresh for ${modifiedFiles.size} modified files`);
            
            try {
                // 1. Import required stores
                const { workspaceStore } = await import('./workspace');
                const { latexStore } = await import('./latex');
                
                // 2. Determine LaTeX ecosystem files
                const LATEX_ECOSYSTEM_PATTERN = /\.(tex|bib|sty|cls|def|cfg|clo)$/i;
                const latexFilesModified: string[] = [];
                const allModifiedPaths: string[] = [];
                
                for (const [path, fileInfo] of modifiedFiles) {
                    allModifiedPaths.push(path);
                    if (LATEX_ECOSYSTEM_PATTERN.test(path)) {
                        latexFilesModified.push(path);
                    }
                }
                
                console.log('Modified files:', allModifiedPaths);
                console.log('LaTeX ecosystem files modified:', latexFilesModified);
                
                // 3. Batch reload all modified files in WorkspaceStore
                console.log('AgentStore: Triggering WorkspaceStore batch file reload');
                for (const path of allModifiedPaths) {
                    const fileInfo = modifiedFiles.get(path)!;
                    try {
                        // Force reload the file to get latest content
                        if (fileInfo.changeType !== 'deleted') {
                            console.log(`AgentStore: Force reloading ${path} after agent modification`);
                            await workspaceStore.loadFile(path, true);
                            
                            // If this file is currently open, we need to trigger a content refresh
                            const workspaceState = workspaceStore.getCurrentState();
                            if (workspaceState.openFiles.includes(path)) {
                                console.log(`AgentStore: File ${path} is open, forcing editor refresh`);
                                // Force the WorkspaceStore to emit a content change event
                                workspaceStore.emitFileChange(path, 'modified');
                            }
                        }
                        
                        // Emit the file system event now that agent run is complete
                        switch (fileInfo.changeType) {
                            case 'created':
                                eventStore.events.fileCreated(path, false, 'agent');
                                break;
                            case 'modified':
                                eventStore.events.fileModified(path, undefined, 'agent');
                                break;
                            case 'deleted':
                                eventStore.events.fileDeleted(path, false, 'agent');
                                break;
                        }
                    } catch (error) {
                        console.warn(`Failed to reload file ${path}:`, error);
                    }
                }
                
                // 4. Trigger LaTeX compilation if needed
                if (latexFilesModified.length > 0) {
                    console.log('AgentStore: Triggering LaTeX compilation after agent file modifications');
                    // Use a small delay to allow file system to settle
                    setTimeout(() => {
                        latexStore.scheduleCompilation('agent_run_complete');
                    }, 500);
                }
                
                // 5. Clear the modified files tracking
                update(state => ({
                    ...state,
                    modifiedFiles: new Map()
                }));
                
                console.log('AgentStore: File coordination completed successfully');
                
            } catch (error) {
                console.error('AgentStore: Error during file coordination:', error);
                
                // Clear tracking even on error to avoid stuck state
                update(state => ({
                    ...state,
                    modifiedFiles: new Map()
                }));
            }
        },

        // Session switching
        async switchSession(newSessionId: string): Promise<void> {
            const currentState = store.getCurrentState();
            
            if (currentState.currentSessionId === newSessionId) {
                return; // Already on this session
            }
            
            // Clean up current session without clearing state
            if (agentEvents) {
                await agentEvents.switchSession(newSessionId);
            }
            
            if (agentFileSync) {
                await agentFileSync.switchSession(newSessionId);
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
            return {
                fileSyncActive: agentFileSync?.isInitialized() || false,
                eventsActive: agentEvents?.getCurrentSessionId() !== null,
                currentSession: store.getCurrentState().currentSessionId
            };
        },

        // EventStore integration methods
        handleAgentFileChange(path: string, changeType: 'created' | 'modified' | 'deleted' | 'renamed'): void {
            console.log(`AgentStore: Duplicate file change ${changeType} on ${path} - skipping (handled by tool operation)`);
            
            // This method is redundant - file changes are already handled by handleAgentToolOperation
            // when the tool operations complete. No need to emit duplicate events.
        },

        handleAgentToolOperation(tool: string, path: string, result: any): void {
            console.log(`AgentStore: Processing tool operation ${tool} on ${path}`);
            
            try {
                // Always emit the agent tool event for tracking
                eventStore.events.agentToolCompleted(
                    store.getCurrentState().currentSessionId || 'unknown',
                    tool,
                    result
                );

                // Skip file system events for read operations - they don't modify files
                if (tool === 'read_file') {
                    console.log(`AgentStore: Skipping file system event for read operation on ${path}`);
                    return;
                }
                
                // Determine the change type from the tool
                let changeType: 'created' | 'modified' | 'deleted';
                switch (tool) {
                    case 'create_file':
                        changeType = 'created';
                        break;
                    case 'delete_file':
                        changeType = 'deleted';
                        break;
                    case 'write_file':
                    case 'update_file':
                        changeType = 'modified';
                        break;
                    default:
                        console.log(`AgentStore: Unknown tool ${tool}, skipping file system event`);
                        return;
                }

                // Track modified files during agent run instead of emitting events immediately
                const currentState = store.getCurrentState();
                if (currentState.isAgentRunning) {
                    update(state => {
                        const newModifiedFiles = new Map(state.modifiedFiles);
                        newModifiedFiles.set(path, {
                            tool,
                            changeType,
                            timestamp: Date.now()
                        });
                        return {
                            ...state,
                            modifiedFiles: newModifiedFiles
                        };
                    });
                    console.log(`AgentStore: Tracked file modification during agent run: ${path} (${changeType})`);
                } else {
                    // Not during agent run, emit events immediately (legacy behavior)
                    switch (changeType) {
                        case 'created':
                            eventStore.events.fileCreated(path, false, 'agent');
                            break;
                        case 'modified':
                            eventStore.events.fileModified(path, undefined, 'agent');
                            break;
                        case 'deleted':
                            eventStore.events.fileDeleted(path, false, 'agent');
                            break;
                    }
                    console.log(`AgentStore: Emitted immediate ${changeType} event for ${path}`);
                }
                
            } catch (error) {
                console.error('AgentStore: Error handling tool operation:', error);
            }
        },

        // Enhanced file operation tracking with EventStore integration
        async handleEnhancedFileOperation(operation: {
            tool: string;
            path: string;
            args?: any;
            result?: string;
        }): Promise<void> {
            const { tool, path, args, result } = operation;
            
            console.log(`AgentStore: Enhanced file operation - ${tool} on ${path}`);
            
            try {
                // Use the standard tool operation handler which emits to EventStore
                store.handleAgentToolOperation(tool, path, result || args);
                
                // Update operation count
                update(state => ({
                    ...state,
                    fileOperationsCount: state.fileOperationsCount + 1
                }));
                
            } catch (error) {
                console.error('AgentStore: Error in enhanced file operation handling:', error);
            }
        },

        // Cleanup
        async destroy(): Promise<void> {
            // Clean up unified event handlers
            if (agentEvents) {
                agentEvents.destroy();
                agentEvents = null;
            }
            
            if (agentFileSync) {
                agentFileSync.destroy();
                agentFileSync = null;
            }
            
            isInitialized = false;
            console.log('Agent store destroyed and cleaned up');
        }
    };

    return store;
}

export const agentStore = createAgentStore();