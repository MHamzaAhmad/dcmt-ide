// Unified Agent Events Hook - Platform-agnostic agent event handling
import { writable, get } from 'svelte/store';
import { useQueryClient, type QueryClient } from '@tanstack/svelte-query';
import { browser } from '$app/environment';
import { isDesktop } from '$lib/utils/platform';
import type { AgentEvent, FileEventData } from '../types';
import { fileSystemKeys } from './useFileSystem';

export type AgentEventCallback = (event: AgentEvent) => void;
export type AgentFileEventCallback = (type: string, event: FileEventData) => void;

export interface AgentEventHookOptions {
    onAgentEvent?: AgentEventCallback;
    onFileEvent?: AgentFileEventCallback;
    enabled?: boolean;
    sessionId?: string;
    queryClient?: QueryClient;
}

/**
 * Unified hook for handling agent events across desktop and web platforms
 * Handles both agent progress events and file change events from agent operations
 */
export function useAgentEvents(options: AgentEventHookOptions = {}) {
    const { onAgentEvent, onFileEvent, enabled = true, sessionId, queryClient: providedQueryClient } = options;
    
    // State management using writable stores
    const isListening = writable(false);
    const connectionStatus = writable<'disconnected' | 'connecting' | 'connected'>('disconnected');
    const lastAgentEvent = writable<AgentEvent | null>(null);
    const lastFileEvent = writable<{ type: string; event: FileEventData } | null>(null);
    const agentEventCount = writable(0);
    const fileEventCount = writable(0);
    const connectionError = writable<string | null>(null);
    
    // Try to get queryClient from context if not provided, but don't fail if not available
    let queryClient: QueryClient | undefined = providedQueryClient;
    try {
        if (!queryClient) {
            queryClient = useQueryClient();
        }
    } catch (error) {
        // Not in component context, that's okay if queryClient was provided
        console.debug('useAgentEvents: Unable to get queryClient from context, query invalidation will be skipped');
    }
    
    let eventCleanup: (() => void) | null = null;
    let isInitialized = false;
    let currentSessionId: string | null = null;

    function handleAgentEvent(event: AgentEvent) {
        console.log('Agent event received:', event);
        
        lastAgentEvent.set(event);
        agentEventCount.update(count => count + 1);
        
        // Call user callback
        onAgentEvent?.(event);
        
        // Handle file-related agent events
        handleAgentFileOperations(event);
    }

    function handleFileEvent(type: string, event: FileEventData) {
        console.log(`File event received (${type}):`, event);
        
        lastFileEvent.set({ type, event });
        fileEventCount.update(count => count + 1);
        
        // Call user callback
        onFileEvent?.(type, event);
        
        // Invalidate related queries
        invalidateFileQueries(type, event);
    }

    function handleAgentFileOperations(event: AgentEvent) {
        // Handle agent events that indicate file operations
        switch (event.type) {
            case 'ToolCompleted':
                // Check if this was a file operation tool
                const fileTools = ['read_file', 'write_file', 'update_file', 'create_file', 'delete_file', 'create_directory'];
                if (fileTools.includes(event.tool)) {
                    // Parse the result to extract file path if possible
                    try {
                        const resultData = JSON.parse(event.result);
                        if (resultData.path) {
                            // Determine event type based on tool
                            let eventType: 'Created' | 'Modified' | 'Deleted' | 'Renamed' = 'Modified';
                            if (event.tool === 'create_file' || event.tool === 'create_directory') {
                                eventType = 'Created';
                            } else if (event.tool === 'delete_file') {
                                eventType = 'Deleted';
                            }
                            
                            // Simulate a file event for query invalidation
                            const fileEvent: FileEventData = {
                                event_type: eventType,
                                path: resultData.path,
                                timestamp: Date.now(),
                                metadata: {
                                    is_directory: event.tool === 'create_directory',
                                    size: resultData.size || undefined,
                                    old_path: undefined,
                                    new_path: undefined
                                }
                            };
                            
                            handleFileEvent(eventType, fileEvent);
                        }
                    } catch (error) {
                        // Result might not be JSON, that's okay
                        console.debug('Agent tool result was not JSON:', event.result);
                    }
                }
                break;
        }
    }

    function invalidateFileQueries(type: string, event: FileEventData) {
        if (!queryClient) return; // Skip if no queryClient available
        
        const path = event.path;
        const pathParts = path.split('/').filter(part => part);
        
        // Invalidate directory tree queries for all parent directories
        for (let i = 0; i <= pathParts.length; i++) {
            const directoryPath = pathParts.slice(0, i).join('/');
            queryClient.invalidateQueries({ 
                queryKey: fileSystemKeys.directoryTree(directoryPath) 
            });
        }
        
        // Handle specific event types
        switch (type) {
            case 'created':
            case 'deleted':
                // Directory structure changed, invalidate parent directories
                break;
                
            case 'modified':
                // Invalidate file content for modified files
                if (!event.metadata?.is_directory) {
                    queryClient.invalidateQueries({ 
                        queryKey: fileSystemKeys.fileContent(path) 
                    });
                }
                break;
                
            case 'renamed':
                // Remove old file from cache
                if (event.metadata?.old_path) {
                    queryClient.removeQueries({ 
                        queryKey: fileSystemKeys.fileContent(event.metadata.old_path) 
                    });
                    
                    // Invalidate old parent directory
                    const oldParentPath = event.metadata.old_path.split('/').slice(0, -1).join('/');
                    queryClient.invalidateQueries({ 
                        queryKey: fileSystemKeys.directoryTree(oldParentPath) 
                    });
                }
                break;
        }
    }

    async function startListening(newSessionId?: string) {
        if (!browser || !enabled) return;
        
        const targetSessionId = newSessionId || sessionId;
        if (!targetSessionId) {
            console.warn('No session ID provided for agent events');
            return;
        }
        
        const currentlyListening = get(isListening);
        if (currentlyListening && currentSessionId === targetSessionId) {
            return; // Already listening to this session
        }
        
        if (isInitialized && currentSessionId !== targetSessionId) {
            // Stop listening to previous session
            await stopListening();
        }
        
        currentSessionId = targetSessionId;
        connectionStatus.set('connecting');
        connectionError.set(null);
        
        try {
            if (isDesktop()) {
                // Desktop: Use Tauri event listeners
                await startDesktopEventListeners(targetSessionId);
            } else {
                // Web: Use WebSocket
                await startWebEventListeners(targetSessionId);
            }
            
            isListening.set(true);
            connectionStatus.set('connected');
            isInitialized = true;
            
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Failed to start event listeners';
            console.error('Failed to start agent event listeners:', errorMessage);
            connectionError.set(errorMessage);
            connectionStatus.set('disconnected');
        }
    }

    async function startDesktopEventListeners(targetSessionId: string) {
        // Import and use the desktop SSE adapter for agent events
        const { desktopAgentSSE } = await import('../adapters/desktop/agentSSE');
        
        // Subscribe to session-specific agent events via SSE
        const agentUnsubscribe = desktopAgentSSE.onSession(targetSessionId, handleAgentEvent);
        
        // Keep Tauri events for file events only (from file watcher)
        const { listen } = await import('@tauri-apps/api/event');
        const fileEventUnlisten = await listen('file-event', (event) => {
            const { event_type, event_data } = event.payload as { event_type: string; event_data: FileEventData };
            handleFileEvent(event_type, event_data);
        });
        
        eventCleanup = () => {
            agentUnsubscribe();
            fileEventUnlisten();
        };
        
        console.log(`Started desktop event listeners for session: ${targetSessionId} (SSE for agent, Tauri events for files)`);
    }

    async function startWebEventListeners(targetSessionId: string) {
        // Import and use the web SSE adapter for agent events
        const { agentSSE } = await import('../adapters/web/agentSSE');
        
        // Subscribe to session-specific agent events via SSE
        const agentUnsubscribe = agentSSE.onSession(targetSessionId, handleAgentEvent);
        
        // Keep WebSocket for file events only
        const { WebFileWatcher } = await import('../adapters/web/fileWebSocket');
        const fileWatcher = new WebFileWatcher();
        
        // Subscribe to file events from WebSocket
        const fileUnsubscribe = fileWatcher.onAny((eventType: string, fileEvent) => {
            handleFileEvent(eventType, fileEvent);
        });
        
        eventCleanup = () => {
            agentUnsubscribe();
            fileUnsubscribe();
            fileWatcher.destroy();
        };
        
        console.log(`Started web event listeners for session: ${targetSessionId} (SSE for agent, WebSocket for files)`);
    }

    async function stopListening() {
        if (eventCleanup) {
            eventCleanup();
            eventCleanup = null;
        }
        
        isListening.set(false);
        connectionStatus.set('disconnected');
        currentSessionId = null;
        isInitialized = false;
        
        console.log('Stopped agent event listeners');
    }

    async function switchSession(newSessionId: string) {
        if (currentSessionId === newSessionId) return;
        
        await stopListening();
        await startListening(newSessionId);
    }

    // Cleanup function
    function destroy() {
        stopListening();
        
        // Reset all stores
        isListening.set(false);
        connectionStatus.set('disconnected');
        lastAgentEvent.set(null);
        lastFileEvent.set(null);
        agentEventCount.set(0);
        fileEventCount.set(0);
        connectionError.set(null);
    }

    return {
        // State
        isListening,
        connectionStatus,
        lastAgentEvent,
        lastFileEvent,
        agentEventCount,
        fileEventCount,
        connectionError,
        
        // Controls
        start: startListening,
        stop: stopListening,
        switchSession,
        destroy,
        
        // Current session info
        getCurrentSessionId: () => currentSessionId
    };
}

/**
 * Simplified hook that just provides agent event listening for a specific session
 */
export function useAgentSession(sessionId: string, onAgentEvent?: AgentEventCallback) {
    return useAgentEvents({
        sessionId,
        onAgentEvent,
        enabled: !!sessionId
    });
}