/**
 * WorkspaceStore - Centralized reactive file management
 * Single source of truth for all file operations, content, and workspace state
 */

import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import { fileSystemApi } from '$lib/api/adapters';
import type { QueryClient } from '@tanstack/svelte-query';
import { useQueryClient } from '@tanstack/svelte-query';

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
    
    // LaTeX specific
    mainLatexFile: string | null;
    latexFiles: string[];
    
    // Status
    isLoading: boolean;
    error: string | null;
    lastActivity: number;
    
    // File operations tracking
    pendingOperations: Map<string, 'reading' | 'writing' | 'creating' | 'deleting'>;
}

function createWorkspaceStore() {
    const initialState: WorkspaceState = {
        isReady: false,
        rootPath: null,
        files: new Map(),
        fileTree: [],
        openFiles: [],
        activeFile: null,
        mainLatexFile: null,
        latexFiles: [],
        isLoading: false,
        error: null,
        lastActivity: Date.now(),
        pendingOperations: new Map()
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

    const latexFilesContent = derived([{ subscribe }], ([$workspace]) => {
        return $workspace.latexFiles
            .map(path => $workspace.files.get(path))
            .filter((file): file is FileContent => file !== undefined);
    });

    // Internal state
    let queryClient: QueryClient | undefined;
    let isInitialized = false;
    let fileWatchers: Map<string, any> = new Map();

    const store = {
        subscribe,
        
        // Derived stores
        openFileContents,
        dirtyFiles,
        latexFilesContent,

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
                
                // Detect main LaTeX file
                await store.detectMainLatexFile();
                
                // Set up file watching if supported
                store.setupFileWatching();

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
                
                // Process files and detect LaTeX files
                const latexFiles: string[] = [];
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
                    
                    if (nodeType === 'file' && node.name.endsWith('.tex')) {
                        latexFiles.push(node.path);
                    }
                    
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
                    latexFiles,
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
            const currentState = get({ subscribe });
            const existingFile = currentState.files.get(path);

            // Return cached if available and not forced
            if (existingFile?.isLoaded && !force) {
                return existingFile;
            }

            // Check if already loading
            if (currentState.pendingOperations.has(path)) {
                throw new Error(`File ${path} is already being loaded`);
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
                store.emitFileChange(path, 'modified');

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

        // LaTeX specific functionality
        async detectMainLatexFile(): Promise<void> {
            const currentState = get({ subscribe });
            
            // Look for main.tex first
            let mainFile = currentState.latexFiles.find(path => 
                path.toLowerCase().includes('main.tex')
            );
            
            // If not found, look for any .tex file with \documentclass
            if (!mainFile && currentState.latexFiles.length > 0) {
                for (const latexPath of currentState.latexFiles) {
                    try {
                        const file = await store.loadFile(latexPath);
                        if (file.content.includes('\\documentclass')) {
                            mainFile = latexPath;
                            break;
                        }
                    } catch (error) {
                        console.warn(`Could not check ${latexPath} for \\documentclass:`, error);
                    }
                }
            }
            
            // Fall back to first .tex file
            if (!mainFile && currentState.latexFiles.length > 0) {
                mainFile = currentState.latexFiles[0];
            }

            if (mainFile) {
                update(state => ({
                    ...state,
                    mainLatexFile: mainFile!
                }));
                
                console.log(`Detected main LaTeX file: ${mainFile}`);
            }
        },

        // Event emission for external systems
        emitFileChange(path: string, changeType: 'created' | 'modified' | 'deleted'): void {
            // Emit custom event for other stores to react to
            if (browser) {
                const event = new CustomEvent('workspace:file-changed', {
                    detail: { path, changeType, timestamp: Date.now() }
                });
                window.dispatchEvent(event);
            }
        },

        // File watching setup
        setupFileWatching(): void {
            // This would integrate with the existing file watcher system
            // For now, we'll rely on agent events and manual refreshes
            console.log('WorkspaceStore: File watching setup completed');
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

        // Cleanup
        async destroy(): Promise<void> {
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