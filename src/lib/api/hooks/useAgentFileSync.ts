// Agent-File Sync Integration Hook - Production-ready file synchronization
import { writable, get } from 'svelte/store';
import { useQueryClient, type QueryClient } from '@tanstack/svelte-query';
import { browser } from '$app/environment';
import { isDesktop } from '$lib/utils/platform';
import type { AgentEvent, FileEventData, AgentToolDefinition } from '../types';
import { fileSystemKeys } from './useFileSystem';
import { useAgentEvents } from './useAgentEvents';
import { useFileWatcher } from './useFileWatcher';

export interface AgentFileSyncOptions {
    sessionId?: string;
    enabled?: boolean;
    onFileChanged?: (path: string, changeType: 'created' | 'modified' | 'deleted' | 'renamed') => void;
    onAgentFileOperation?: (tool: string, path: string, result: any) => void;
    debounceMs?: number;
    queryClient?: QueryClient;
}

export interface FileSyncState {
    isActive: boolean;
    pendingOperations: Map<string, AgentFileOperation>;
    completedOperations: AgentFileOperation[];
    lastSyncTime: number | null;
    syncErrors: string[];
}

export interface AgentFileOperation {
    id: string;
    tool: string;
    path: string;
    sessionId: string;
    startTime: number;
    endTime?: number;
    result?: any;
    error?: string;
    status: 'pending' | 'executing' | 'completed' | 'failed';
}

/**
 * Production-ready hook for synchronizing agent file operations with the UI
 * Handles real-time file updates, query invalidation, and operation tracking
 */
export function useAgentFileSync(options: AgentFileSyncOptions = {}) {
    const {
        sessionId,
        enabled = true,
        onFileChanged,
        onAgentFileOperation,
        debounceMs = 100,
        queryClient: providedQueryClient
    } = options;

    // State management
    const syncState = writable<FileSyncState>({
        isActive: false,
        pendingOperations: new Map(),
        completedOperations: [],
        lastSyncTime: null,
        syncErrors: []
    });

    const operationCount = writable(0);
    const lastOperation = writable<AgentFileOperation | null>(null);
    const isProcessing = writable(false);

    // Try to get queryClient from context if not provided, but don't fail if not available
    let queryClient: QueryClient | undefined = providedQueryClient;
    try {
        if (!queryClient) {
            queryClient = useQueryClient();
        }
    } catch (error) {
        // Not in component context, that's okay if queryClient was provided
        console.debug('useAgentFileSync: Unable to get queryClient from context, query invalidation will be skipped');
    }
    
    // File tools that trigger file system changes
    // NOTE: read_file is excluded as it doesn't modify files
    const FILE_TOOLS = new Set([
        'write_file', 
        'update_file',
        'create_file',
        'delete_file',
        'create_directory',
        'rename_file',
        'copy_file',
        'move_file'
    ]);

    // Debounce map for query invalidation
    const debounceTimers = new Map<string, ReturnType<typeof setTimeout>>();

    let cleanup: (() => void) | null = null;
    let isInitialized = false;

    function generateOperationId(): string {
        return `op-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    }

    function handleAgentEvent(event: AgentEvent) {
        if (!enabled || !sessionId) return;

        switch (event.type) {
            case 'ToolExecuting':
                handleToolExecuting(event);
                break;
            case 'ToolCompleted':
                handleToolCompleted(event);
                break;
            case 'Error':
                handleAgentError(event);
                break;
            case 'JobComplete':
                handleJobComplete();
                break;
        }
    }

    function handleToolExecuting(event: { tool: string }) {
        if (!FILE_TOOLS.has(event.tool)) return;

        const operation: AgentFileOperation = {
            id: generateOperationId(),
            tool: event.tool,
            path: '', // Will be updated when completed
            sessionId: sessionId!,
            startTime: Date.now(),
            status: 'executing'
        };

        syncState.update(state => {
            const newPending = new Map(state.pendingOperations);
            newPending.set(operation.id, operation);
            return {
                ...state,
                pendingOperations: newPending,
                isActive: true
            };
        });

        isProcessing.set(true);
        operationCount.update(count => count + 1);

        console.log(`Agent file operation started: ${event.tool}`, operation);
    }

    function handleToolCompleted(event: { tool: string; result: string }) {
        if (!FILE_TOOLS.has(event.tool)) return;

        const currentState = get(syncState);
        
        // Find the pending operation for this tool
        let operationToComplete: AgentFileOperation | null = null;
        for (const [id, op] of currentState.pendingOperations) {
            if (op.tool === event.tool && op.status === 'executing') {
                operationToComplete = { ...op };
                break;
            }
        }

        if (!operationToComplete) {
            // Create a new operation if we missed the start event
            operationToComplete = {
                id: generateOperationId(),
                tool: event.tool,
                path: '',
                sessionId: sessionId!,
                startTime: Date.now() - 1000, // Estimate start time
                status: 'executing'
            };
        }

        // Parse the result to extract file information
        const fileInfo = parseToolResult(event.tool, event.result);
        
        operationToComplete.endTime = Date.now();
        operationToComplete.result = event.result;
        operationToComplete.path = fileInfo.path || operationToComplete.path;
        operationToComplete.status = 'completed';

        syncState.update(state => {
            const newPending = new Map(state.pendingOperations);
            newPending.delete(operationToComplete!.id);
            
            const newCompleted = [...state.completedOperations, operationToComplete!].slice(-50); // Keep last 50
            
            return {
                ...state,
                pendingOperations: newPending,
                completedOperations: newCompleted,
                lastSyncTime: Date.now(),
                isActive: newPending.size > 0
            };
        });

        lastOperation.set(operationToComplete);
        
        // Trigger file system updates
        if (fileInfo.path) {
            handleFileSystemChange(fileInfo.path, fileInfo.changeType, fileInfo);
            onAgentFileOperation?.(event.tool, fileInfo.path, event.result);
        }

        // Update processing state
        if (currentState.pendingOperations.size <= 1) {
            isProcessing.set(false);
        }

        console.log(`Agent file operation completed: ${event.tool}`, operationToComplete);
    }

    function handleAgentError(event: { message: string }) {
        syncState.update(state => ({
            ...state,
            syncErrors: [...state.syncErrors.slice(-9), event.message], // Keep last 10 errors
            isActive: false
        }));

        isProcessing.set(false);

        console.error('Agent file sync error:', event.message);
    }

    function handleJobComplete() {
        syncState.update(state => ({
            ...state,
            isActive: false
        }));

        isProcessing.set(false);
    }

    function parseToolResult(tool: string, result: string): {
        path?: string;
        changeType: 'created' | 'modified' | 'deleted' | 'renamed';
        oldPath?: string;
        size?: number;
        isDirectory?: boolean;
    } {
        // First, try to extract path from the tool arguments if visible in result
        let path: string | undefined;
        let oldPath: string | undefined;
        
        try {
            // Try to parse as JSON first
            const parsed = JSON.parse(result);
            
            if (parsed.path || parsed.file_path || parsed.filename) {
                path = parsed.path || parsed.file_path || parsed.filename;
                oldPath = parsed.old_path || parsed.old_name;
                
                return {
                    path,
                    changeType: determineChangeType(tool),
                    oldPath,
                    size: parsed.size,
                    isDirectory: parsed.is_directory || tool === 'create_directory'
                };
            }
        } catch {
            // Result is not JSON, try to extract path from text
        }

        // Enhanced patterns for tool results
        const pathPatterns = [
            // Common success messages
            /Successfully (?:wrote|created|updated|deleted|renamed) (?:file |directory )?['""]?([^'""]+)['""]?/i,
            /File ['""]?([^'""]+)['""]? (?:written|created|updated|deleted|renamed)/i,
            /(?:Writing|Creating|Updating|Deleting) (?:file |directory )?['""]?([^'""]+)['""]?/i,
            // Path references
            /(?:file|path|filename)[:=]\s*['""]?([^'""]+)['""]?/i,
            // Direct path mentions
            /['""]([\/\w\-\.]+\.\w+)['""]/, // Files with extensions
            /['""]([\/\w\-\.]+)['""]/, // Any path in quotes
            // Tool-specific patterns
            /read_file:\s*([^\s]+)/i,
            /write_file:\s*([^\s]+)/i,
            // Fallback to any path-like string
            /(?:^|\s)([\/\w\-\.]+\.[\w]+)(?:\s|$)/,
            /(?:^|\s)([\w\-\.\/]+)(?:\s|$)/
        ];

        for (const pattern of pathPatterns) {
            const match = result.match(pattern);
            if (match && match[1]) {
                path = match[1].trim();
                // Remove trailing punctuation if any
                path = path.replace(/[,;.!]$/, '');
                break;
            }
        }

        // Log for debugging
        if (path) {
            console.log(`Extracted path from ${tool} result: ${path}`);
        }

        return {
            path,
            changeType: determineChangeType(tool)
        };
    }

    function determineChangeType(tool: string): 'created' | 'modified' | 'deleted' | 'renamed' {
        switch (tool) {
            case 'create_file':
            case 'create_directory':
            case 'copy_file':
                return 'created';
            case 'write_file':
            case 'update_file':
                return 'modified';
            case 'delete_file':
                return 'deleted';
            case 'rename_file':
            case 'move_file':
                return 'renamed';
            default:
                return 'modified';
        }
    }

    function handleFileSystemChange(
        path: string, 
        changeType: 'created' | 'modified' | 'deleted' | 'renamed',
        info: any
    ) {
        // Clear any existing debounce timer for this path
        const existingTimer = debounceTimers.get(path);
        if (existingTimer) {
            clearTimeout(existingTimer);
        }

        // Set up debounced query invalidation
        const timer = setTimeout(() => {
            invalidateQueriesForPath(path, changeType, info);
            debounceTimers.delete(path);
            
            // Trigger callback
            onFileChanged?.(path, changeType);
        }, debounceMs);

        debounceTimers.set(path, timer);
    }

    function invalidateQueriesForPath(
        path: string,
        changeType: 'created' | 'modified' | 'deleted' | 'renamed', 
        info: any
    ) {
        if (!queryClient) return; // Skip if no queryClient available
        
        console.log(`Invalidating queries for ${changeType} on path: ${path}`);
        
        const pathParts = path.split('/').filter(part => part);
        
        // Invalidate directory tree queries for all parent directories
        for (let i = 0; i <= pathParts.length; i++) {
            const directoryPath = pathParts.slice(0, i).join('/');
            console.log(`Invalidating directory tree query for: ${directoryPath}`);
            queryClient.invalidateQueries({ 
                queryKey: fileSystemKeys.directoryTree(directoryPath) 
            });
        }

        // Always invalidate file content for any file operation
        if (!info.isDirectory) {
            console.log(`Invalidating file content query for: ${path}`);
            queryClient.invalidateQueries({ 
                queryKey: fileSystemKeys.fileContent(path) 
            });
            
            // Also remove from cache to force fresh fetch
            queryClient.removeQueries({
                queryKey: fileSystemKeys.fileContent(path)
            });
        }

        // Handle specific change types
        switch (changeType) {
            case 'created':
            case 'deleted':
                // Invalidate entire file system structure to be safe
                queryClient.invalidateQueries({
                    predicate: (query) => {
                        const key = query.queryKey as string[];
                        return key[0] === 'filesystem' || key[0] === 'directory-tree';
                    }
                });
                break;
                
            case 'modified':
                // For modified files, also invalidate any related queries
                if (!info.isDirectory) {
                    // Invalidate any queries that might contain this file path
                    queryClient.invalidateQueries({
                        predicate: (query) => {
                            const key = query.queryKey as string[];
                            return key.some(part => typeof part === 'string' && part.includes(path));
                        }
                    });
                }
                break;
                
            case 'renamed':
                // Remove old file from cache
                if (info.oldPath) {
                    queryClient.removeQueries({ 
                        queryKey: fileSystemKeys.fileContent(info.oldPath) 
                    });
                    
                    // Invalidate old parent directory
                    const oldParentPath = info.oldPath.split('/').slice(0, -1).join('/');
                    queryClient.invalidateQueries({ 
                        queryKey: fileSystemKeys.directoryTree(oldParentPath) 
                    });
                }
                break;
        }

        // Force a global file system refetch for critical operations
        if (['created', 'deleted', 'renamed'].includes(changeType)) {
            setTimeout(() => {
                queryClient.refetchQueries({
                    predicate: (query) => {
                        const key = query.queryKey as string[];
                        return key[0] === 'filesystem';
                    }
                });
            }, 100);
        }

        console.log(`Completed query invalidation for ${changeType} operation on: ${path}`);
    }

    function handleFileEvent(type: string, event: FileEventData) {
        // Handle direct file events from file watcher
        // This complements agent operations with manual file changes
        const changeType = type as 'created' | 'modified' | 'deleted' | 'renamed';
        handleFileSystemChange(event.path, changeType, {
            oldPath: event.metadata?.old_path,
            isDirectory: event.metadata?.is_directory
        });
    }

    async function start() {
        if (!browser || !enabled || isInitialized) return;

        if (!sessionId) {
            return;
        }

        try {
            // Set up agent event handling
            const agentEvents = useAgentEvents({
                sessionId,
                onAgentEvent: handleAgentEvent,
                onFileEvent: handleFileEvent,
                enabled: true,
                queryClient
            });

            await agentEvents.start();

            // Set up file watcher integration
            const fileWatcher = useFileWatcher(handleFileEvent, true, queryClient);
            fileWatcher.start();

            cleanup = () => {
                agentEvents.destroy();
                fileWatcher.destroy();
                
                // Clear debounce timers
                debounceTimers.forEach(timer => clearTimeout(timer));
                debounceTimers.clear();
            };

            isInitialized = true;
            
            syncState.update(state => ({
                ...state,
                isActive: true,
                lastSyncTime: Date.now(),
                syncErrors: []
            }));

            console.log(`Agent file sync started for session: ${sessionId}`);
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Unknown error';
            console.error('Failed to start agent file sync:', errorMessage);
            
            syncState.update(state => ({
                ...state,
                syncErrors: [...state.syncErrors, errorMessage]
            }));
        }
    }

    async function stop() {
        if (cleanup) {
            cleanup();
            cleanup = null;
        }

        isInitialized = false;
        
        syncState.update(state => ({
            ...state,
            isActive: false,
            pendingOperations: new Map(),
            lastSyncTime: Date.now()
        }));

        isProcessing.set(false);
        
        console.log('Agent file sync stopped');
    }

    async function switchSession(newSessionId: string) {
        if (newSessionId === sessionId) return;
        
        await stop();
        // Update sessionId would need to be done externally
        await start();
    }

    function clearErrors() {
        syncState.update(state => ({
            ...state,
            syncErrors: []
        }));
    }

    function clearCompletedOperations() {
        syncState.update(state => ({
            ...state,
            completedOperations: []
        }));
        
        operationCount.set(0);
    }

    function getOperationHistory(limit: number = 50): AgentFileOperation[] {
        const state = get(syncState);
        return state.completedOperations.slice(-limit);
    }

    function getCurrentOperations(): AgentFileOperation[] {
        const state = get(syncState);
        return Array.from(state.pendingOperations.values());
    }

    // Cleanup function
    function destroy() {
        stop();
        
        // Reset stores
        syncState.set({
            isActive: false,
            pendingOperations: new Map(),
            completedOperations: [],
            lastSyncTime: null,
            syncErrors: []
        });
        
        operationCount.set(0);
        lastOperation.set(null);
        isProcessing.set(false);
    }

    return {
        // State
        syncState,
        operationCount,
        lastOperation,
        isProcessing,
        
        // Controls
        start,
        stop,
        switchSession,
        destroy,
        
        // Utilities
        clearErrors,
        clearCompletedOperations,
        getOperationHistory,
        getCurrentOperations,
        
        // Status
        isInitialized: () => isInitialized,
        getSessionId: () => sessionId
    };
}

/**
 * Simplified hook that just provides automatic file sync for an agent session
 */
export function useAutoFileSync(sessionId: string, enabled: boolean = true) {
    return useAgentFileSync({
        sessionId,
        enabled,
        debounceMs: 150 // Slightly longer debounce for auto mode
    });
}