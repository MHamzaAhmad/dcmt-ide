/**
 * WorkspaceStore - Centralized reactive file management
 * Single source of truth for all file operations, content, and workspace state
 */

import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import { fileSystemApi } from '$lib/api/adapters';
import type { QueryClient } from '@tanstack/svelte-query';
import { useQueryClient } from '@tanstack/svelte-query';
import { eventStore } from './events';

export interface FileContent {
    path: string;
    content: string;
    size: number;
    lastModified: number;
    isLoaded: boolean;
    isDirty: boolean;
    originalContent: string;
    encoding?: string;
}

export interface FileTreeNode {
    path: string;
    name: string;
    type: 'file' | 'directory';
    size?: number;
    lastModified?: number;
    children?: FileTreeNode[];
    isExpanded?: boolean;
}

export interface WorkspaceState {
    // Core state
    isReady: boolean;
    rootPath: string | null;
    
    // File management
    files: Map<string, FileContent>;
    fileTree: FileTreeNode[];
    openFiles: string[];
    activeFile: string | null;
    
    
    // Status
    isLoading: boolean;
    error: string | null;
    lastActivity: number;
    
    // File operations tracking
    pendingOperations: Map<string, 'reading' | 'writing' | 'creating' | 'deleting'>;
    
    // File watching control
    isFileWatchingPaused: boolean;
}

function createWorkspaceStore() {
    const initialState: WorkspaceState = {
        isReady: false,
        rootPath: null,
        files: new Map(),
        fileTree: [],
        openFiles: [],
        activeFile: null,
        isLoading: false,
        error: null,
        lastActivity: Date.now(),
        pendingOperations: new Map(),
        isFileWatchingPaused: false
    };

    const { subscribe, set, update } = writable<WorkspaceState>(initialState);

    // Derived stores for common queries
    const openFileContents = derived([{ subscribe }], ([$workspace]) => {
        return $workspace.openFiles
            .map(path => $workspace.files.get(path))
            .filter((file): file is FileContent => file !== undefined);
    });

    const dirtyFiles = derived([{ subscribe }], ([$workspace]) => {
        const dirtyFiles: FileContent[] = [];
        $workspace.files.forEach(file => {
            if (file.isDirty) {
                dirtyFiles.push(file);
            }
        });
        return dirtyFiles;
    });


    // Internal state
    let queryClient: QueryClient | undefined;
    let isInitialized = false;
    let fileWatchers: Map<string, any> = new Map();
    let eventUnsubscribe: (() => void) | null = null;

    const store = {
        subscribe,
        
        // Derived stores
        openFileContents,
        dirtyFiles,

        // Initialization
        async initialize(rootPath: string = '', providedQueryClient?: QueryClient): Promise<void> {
            if (!browser || isInitialized) return;

            // Set query client
            queryClient = providedQueryClient;
            if (!queryClient) {
                try {
                    queryClient = useQueryClient();
                } catch (e) {
                    console.debug('WorkspaceStore: No queryClient available, some features may be limited');
                }
            }

            update(state => ({
                ...state,
                isLoading: true,
                error: null,
                rootPath
            }));

            try {
                // Load initial file tree
                await store.refreshFileTree();
                
                // Set up file watching if supported
                store.setupFileWatching();
                
                // Subscribe to EventStore for agent file operations
                store.subscribeToAgentEvents();

                update(state => ({
                    ...state,
                    isReady: true,
                    isLoading: false,
                    lastActivity: Date.now()
                }));

                isInitialized = true;
                console.log('WorkspaceStore initialized successfully');

            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : 'Failed to initialize workspace';
                console.error('WorkspaceStore initialization failed:', errorMessage);
                
                update(state => ({
                    ...state,
                    isLoading: false,
                    error: errorMessage
                }));
            }
        },

        // File tree management
        async refreshFileTree(): Promise<void> {
            try {
                const currentState = get({ subscribe });
                const files = await fileSystemApi.getDirectoryTree(currentState.rootPath || '');
                
                // Process files
                const processNode = (node: any): FileTreeNode => {
                    // Map API response format to internal format
                    const nodeType = node.file_type === 'File' ? 'file' : 
                                   node.file_type === 'Directory' ? 'directory' : 
                                   node.type; // fallback to direct type
                    
                    const result: FileTreeNode = {
                        path: node.path,
                        name: node.name,
                        type: nodeType as 'file' | 'directory',
                        size: node.size,
                        lastModified: node.modified || node.lastModified
                    };
                    
                    if (node.children) {
                        result.children = node.children.map(processNode);
                    }
                    
                    return result;
                };

                // Handle API response - if it's a directory with children, process the children
                const fileTree = files.children ? files.children.map(processNode) : 
                               Array.isArray(files) ? files.map(processNode) : 
                               [processNode(files)];

                update(state => ({
                    ...state,
                    fileTree,
                    lastActivity: Date.now()
                }));

            } catch (error) {
                console.error('Failed to refresh file tree:', error);
                update(state => ({
                    ...state,
                    error: error instanceof Error ? error.message : 'Failed to refresh file tree'
                }));
            }
        },

        // File content management
        async loadFile(path: string, force: boolean = false): Promise<FileContent> {
            // Skip binary files - workspace store only handles text files
            const BINARY_FILE_PATTERN = /\.(pdf|png|jpg|jpeg|gif|bmp|ico|exe|bin|zip|tar|gz|7z)$/i;
            if (BINARY_FILE_PATTERN.test(path)) {
                console.log(`WorkspaceStore: Skipping binary file ${path} - not a text file`);
                throw new Error(`Cannot load binary file as text: ${path}`);
            }
            
            const currentState = get({ subscribe });
            const existingFile = currentState.files.get(path);

            // Return cached if available and not forced
            if (existingFile?.isLoaded && !force) {
                return existingFile;
            }

            // Check if already loading - wait for existing operation instead of throwing
            if (currentState.pendingOperations.has(path)) {
                console.log(`File ${path} is already being loaded, waiting for completion...`);
                // Wait for the existing operation to complete by polling
                return new Promise((resolve, reject) => {
                    const checkInterval = setInterval(() => {
                        const state = get({ subscribe });
                        if (!state.pendingOperations.has(path)) {
                            clearInterval(checkInterval);
                            const file = state.files.get(path);
                            if (file?.isLoaded) {
                                resolve(file);
                            } else {
                                reject(new Error(`File ${path} failed to load`));
                            }
                        }
                    }, 50); // Check every 50ms
                    
                    // Timeout after 5 seconds
                    setTimeout(() => {
                        clearInterval(checkInterval);
                        reject(new Error(`Timeout waiting for file ${path} to load`));
                    }, 5000);
                });
            }

            // Mark as loading
            update(state => {
                const newOperations = new Map(state.pendingOperations);
                newOperations.set(path, 'reading');
                return {
                    ...state,
                    pendingOperations: newOperations
                };
            });

            try {
                const fileData = await fileSystemApi.readFileContent(path);
                
                const fileContent: FileContent = {
                    path,
                    content: fileData.content,
                    size: fileData.size,
                    lastModified: fileData.modified || Date.now(),
                    isLoaded: true,
                    isDirty: false,
                    originalContent: fileData.content,
                };

                update(state => {
                    const newFiles = new Map(state.files);
                    newFiles.set(path, fileContent);
                    
                    const newOperations = new Map(state.pendingOperations);
                    newOperations.delete(path);
                    
                    return {
                        ...state,
                        files: newFiles,
                        pendingOperations: newOperations,
                        lastActivity: Date.now()
                    };
                });

                return fileContent;

            } catch (error) {
                // Clear loading state
                update(state => {
                    const newOperations = new Map(state.pendingOperations);
                    newOperations.delete(path);
                    return {
                        ...state,
                        pendingOperations: newOperations,
                        error: error instanceof Error ? error.message : `Failed to load file: ${path}`
                    };
                });
                throw error;
            }
        },

        async saveFile(path: string): Promise<void> {
            const currentState = get({ subscribe });
            const file = currentState.files.get(path);
            
            if (!file) {
                throw new Error(`File ${path} not loaded`);
            }

            if (!file.isDirty) {
                return; // Nothing to save
            }

            // Mark as saving
            update(state => {
                const newOperations = new Map(state.pendingOperations);
                newOperations.set(path, 'writing');
                return {
                    ...state,
                    pendingOperations: newOperations
                };
            });

            try {
                await fileSystemApi.writeFileContent(path, file.content);
                
                // Update file state
                update(state => {
                    const newFiles = new Map(state.files);
                    const updatedFile = { 
                        ...file, 
                        isDirty: false, 
                        originalContent: file.content,
                        lastModified: Date.now()
                    };
                    newFiles.set(path, updatedFile);
                    
                    const newOperations = new Map(state.pendingOperations);
                    newOperations.delete(path);
                    
                    return {
                        ...state,
                        files: newFiles,
                        pendingOperations: newOperations,
                        lastActivity: Date.now()
                    };
                });

                // Emit workspace change event for external systems
                store.emitFileChange(path, 'modified', 'user');

            } catch (error) {
                update(state => {
                    const newOperations = new Map(state.pendingOperations);
                    newOperations.delete(path);
                    return {
                        ...state,
                        pendingOperations: newOperations,
                        error: error instanceof Error ? error.message : `Failed to save file: ${path}`
                    };
                });
                throw error;
            }
        },

        /**
         * Commit a successful save performed elsewhere (e.g., SaveController)
         * - Updates file state to not dirty, refreshes originalContent and lastModified
         * - Emits a file modified event with provided source (defaults to 'user')
         * - Does NOT perform any IO
         */
        commitSave(path: string, content?: string, source: 'agent' | 'user' | 'watcher' = 'user'): void {
            const currentState = get({ subscribe });
            const file = currentState.files.get(path);
            if (!file) return;

            const newContent = content ?? file.content;
            update(state => {
                const newFiles = new Map(state.files);
                const updatedFile: FileContent = {
                    ...file,
                    content: newContent,
                    isDirty: false,
                    originalContent: newContent,
                    lastModified: Date.now()
                };
                newFiles.set(path, updatedFile);
                return {
                    ...state,
                    files: newFiles,
                    lastActivity: Date.now()
                };
            });

            // Emit event for downstream consumers
            store.emitFileChange(path, 'modified', source);
        },

        // File content updates (from editor)
        updateFileContent(path: string, content: string): void {
            update(state => {
                const existingFile = state.files.get(path);
                if (!existingFile) {
                    console.warn(`Attempted to update non-existent file: ${path}`);
                    return state;
                }

                const updatedFile: FileContent = {
                    ...existingFile,
                    content,
                    isDirty: content !== existingFile.originalContent,
                    lastModified: Date.now()
                };

                const newFiles = new Map(state.files);
                newFiles.set(path, updatedFile);

                return {
                    ...state,
                    files: newFiles,
                    lastActivity: Date.now()
                };
            });
        },

        // Agent file operations integration
        async handleAgentFileOperation(tool: string, path: string, content?: string): Promise<void> {
            console.log(`WorkspaceStore: Handling agent operation ${tool} on ${path}`);

            switch (tool) {
                case 'read_file':
                    await store.loadFile(path, true);
                    break;
                    
                case 'write_file':
                case 'update_file':
                    if (content !== undefined) {
                        const fileContent: FileContent = {
                            path,
                            content,
                            size: content.length,
                            lastModified: Date.now(),
                            isLoaded: true,
                            isDirty: false,
                            originalContent: content
                        };

                        update(state => {
                            const newFiles = new Map(state.files);
                            newFiles.set(path, fileContent);
                            return {
                                ...state,
                                files: newFiles,
                                lastActivity: Date.now()
                            };
                        });

                        store.emitFileChange(path, 'modified');
                    }
                    break;
                    
                case 'create_file':
                    await store.refreshFileTree();
                    if (content !== undefined) {
                        await store.handleAgentFileOperation('write_file', path, content);
                    }
                    break;
                    
                case 'delete_file':
                    update(state => {
                        const newFiles = new Map(state.files);
                        newFiles.delete(path);
                        const newOpenFiles = state.openFiles.filter(f => f !== path);
                        const newActiveFile = state.activeFile === path ? 
                            (newOpenFiles.length > 0 ? newOpenFiles[0] : null) : state.activeFile;
                        
                        return {
                            ...state,
                            files: newFiles,
                            openFiles: newOpenFiles,
                            activeFile: newActiveFile
                        };
                    });
                    await store.refreshFileTree();
                    store.emitFileChange(path, 'deleted');
                    break;
            }
        },

        // Open files management
        async openFile(path: string): Promise<void> {
            const currentState = get({ subscribe });
            
            // Load file if not loaded
            if (!currentState.files.has(path)) {
                await store.loadFile(path);
            }
            
            // Add to open files if not already open
            if (!currentState.openFiles.includes(path)) {
                update(state => ({
                    ...state,
                    openFiles: [...state.openFiles, path]
                }));
            }

            store.setActiveFile(path);
        },

        closeFile(path: string): void {
            update(state => {
                const newOpenFiles = state.openFiles.filter(f => f !== path);
                const newActiveFile = state.activeFile === path ? 
                    (newOpenFiles.length > 0 ? newOpenFiles[newOpenFiles.length - 1] : null) : 
                    state.activeFile;
                
                return {
                    ...state,
                    openFiles: newOpenFiles,
                    activeFile: newActiveFile
                };
            });
        },

        setActiveFile(path: string | null): void {
            update(state => ({
                ...state,
                activeFile: path,
                lastActivity: Date.now()
            }));
        },


        // Event emission for external systems
        emitFileChange(path: string, changeType: 'created' | 'modified' | 'deleted', source?: 'agent' | 'user' | 'watcher'): void {
            const eventSource = source || 'user';
            console.log(`WorkspaceStore: Emitting ${changeType} event for ${path} from ${eventSource}`);
            
            // Get file size if available
            const currentState = get({ subscribe });
            const file = currentState.files.get(path);
            const size = file?.size;
            
            // Emit to EventStore - unified event system
            switch (changeType) {
                case 'created':
                    eventStore.events.fileCreated(path, false, eventSource);
                    break;
                case 'modified':
                    eventStore.events.fileModified(path, size, eventSource);
                    break;
                case 'deleted':
                    eventStore.events.fileDeleted(path, false, eventSource);
                    break;
            }
        },

        // File watching setup
        setupFileWatching(): void {
            // This would integrate with the existing file watcher system
            // For now, we'll rely on agent events and manual refreshes
            console.log('WorkspaceStore: File watching setup completed');
        },

        // Subscribe to EventStore for agent file operations
        subscribeToAgentEvents(): void {
            // Prevent duplicate subscriptions
            if (eventUnsubscribe) {
                console.log('WorkspaceStore: Already subscribed to events, skipping duplicate subscription');
                return;
            }
            
            // Create event stream for file system events from agents
            const agentFileEvents = eventStore.createFileSystemEventStream();
            
            // React to agent file changes
            const fileUnsubscribe = agentFileEvents.subscribe((events: any) => {
                const latestEvent = events[events.length - 1];
                if (latestEvent && latestEvent.payload.path) {
                    const source = latestEvent.payload.source;
                    const subtype = latestEvent.subtype;
                    
                    // Check if file watching is paused
                    const currentState = get({ subscribe });
                    if (currentState.isFileWatchingPaused) {
                        console.log(`WorkspaceStore: File watching paused, ignoring ${source} ${subtype} on ${latestEvent.payload.path}`);
                        return;
                    }
                    
                    // Only react to meaningful file changes from external sources
                    if (source === 'agent' || source === 'watcher') {
                        // Don't reload for read operations - they don't change the file
                        if (source === 'agent' && (subtype === 'file_read' || subtype === 'file_accessed')) {
                            console.log(`WorkspaceStore: Skipping reload for agent read operation on ${latestEvent.payload.path}`);
                            return;
                        }
                        
                        console.log(`WorkspaceStore: Reacting to ${source} ${subtype} on ${latestEvent.payload.path}`);
                        
                        const filePath = latestEvent.payload.path;
                        
                        // Only reload files that are actually in our store (opened for editing)
                        // Don't try to load files that aren't currently open - this prevents feedback loops
                        if (currentState.files.has(filePath) && 
                            (subtype === 'file_modified' || subtype === 'file_created')) {
                            console.log(`WorkspaceStore: Reloading ${filePath} after ${source} ${subtype}`);
                            // Force reload the file to get updated content
                            store.loadFile(filePath, true).catch(error => {
                                console.warn(`Failed to reload ${filePath} after ${source} operation:`, error);
                            });
                        } else {
                            console.log(`WorkspaceStore: File ${filePath} not in store or not a text modification - skipping reload`);
                        }
                    }
                }
            });
            
            // Store unsubscribe function
            eventUnsubscribe = fileUnsubscribe;
            
            console.log('WorkspaceStore: Subscribed to agent file events');
        },

        // Utilities
        getFile(path: string): FileContent | undefined {
            const state = get({ subscribe });
            return state.files.get(path);
        },

        isFileOpen(path: string): boolean {
            const state = get({ subscribe });
            return state.openFiles.includes(path);
        },

        isDirty(path?: string): boolean {
            const state = get({ subscribe });
            if (path) {
                return state.files.get(path)?.isDirty || false;
            }
            // Check if any file is dirty
            for (const file of state.files.values()) {
                if (file.isDirty) return true;
            }
            return false;
        },

        getCurrentState(): WorkspaceState {
            return get({ subscribe });
        },

        // File watching control
        pauseFileWatching(): void {
            update(state => ({
                ...state,
                isFileWatchingPaused: true
            }));
            console.log('WorkspaceStore: File watching paused');
        },

        resumeFileWatching(): void {
            update(state => ({
                ...state,
                isFileWatchingPaused: false
            }));
            console.log('WorkspaceStore: File watching resumed');
        },

        // Batch file operations
        async batchReloadModifiedFiles(filePaths: string[]): Promise<void> {
            if (filePaths.length === 0) {
                console.log('WorkspaceStore: No files to batch reload');
                return;
            }

            console.log(`WorkspaceStore: Batch reloading ${filePaths.length} files:`, filePaths);
            
            const reloadPromises = filePaths.map(async (filePath) => {
                try {
                    // Skip binary files - workspace store only handles text files
                    const BINARY_FILE_PATTERN = /\.(pdf|png|jpg|jpeg|gif|bmp|ico|exe|bin|zip|tar|gz|7z)$/i;
                    if (BINARY_FILE_PATTERN.test(filePath)) {
                        console.log(`WorkspaceStore: Skipping binary file ${filePath} in batch reload`);
                        return;
                    }
                    
                    // Force reload the file to get latest content
                    await store.loadFile(filePath, true);
                    console.log(`WorkspaceStore: Batch reloaded ${filePath}`);
                    
                    // If this file is currently open, trigger a content change event
                    const currentState = get({ subscribe });
                    if (currentState.openFiles.includes(filePath)) {
                        console.log(`WorkspaceStore: File ${filePath} is open, emitting content change`);
                        store.emitFileChange(filePath, 'modified', 'agent');
                    }
                    
                } catch (error) {
                    const errorMessage = error instanceof Error ? error.message : String(error);
                    if (errorMessage.includes('Cannot load binary file')) {
                        console.log(`WorkspaceStore: Skipping binary file ${filePath} - not a text file`);
                    } else {
                        console.warn(`WorkspaceStore: Failed to batch reload ${filePath}:`, error);
                    }
                }
            });

            // Wait for all files to reload
            await Promise.all(reloadPromises);
            console.log('WorkspaceStore: Batch file reload completed');

            // Update last activity
            update(state => ({
                ...state,
                lastActivity: Date.now()
            }));
        },

        // Cleanup
        async destroy(): Promise<void> {
            // Unsubscribe from EventStore events
            if (eventUnsubscribe) {
                eventUnsubscribe();
                eventUnsubscribe = null;
            }
            
            // Clean up file watchers
            fileWatchers.forEach(watcher => {
                if (watcher && typeof watcher.close === 'function') {
                    watcher.close();
                }
            });
            fileWatchers.clear();
            
            // Reset state
            set(initialState);
            isInitialized = false;
            
            console.log('WorkspaceStore destroyed and cleaned up');
        }
    };

    return store;
}

export const workspaceStore = createWorkspaceStore();