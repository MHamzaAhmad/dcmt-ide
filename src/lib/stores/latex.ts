/**
 * LaTeXStore - Reactive LaTeX compilation management
 * Automatically compiles LaTeX files when they change and manages compilation lifecycle
 */

import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import { workspaceStore } from './workspace';
import { platformApi } from '$lib/api/adapters';
import { LaTeXProvider } from '$lib/api/types';
import { eventStore } from './events';

export interface LaTeXCompilationResult {
    success: boolean;
    outputFile?: string;
    pdfPath?: string;
    errors: string[];
    warnings: string[];
    duration: number;
    timestamp: number;
}

export interface LaTeXState {
    // Core state
    isReady: boolean;
    mainFile: string | null;
    
    // Compilation status
    compilationStatus: 'idle' | 'queued' | 'compiling' | 'success' | 'error';
    isCompiling: boolean;
    autoCompile: boolean;
    
    // Results
    lastCompilation: LaTeXCompilationResult | null;
    compilationHistory: LaTeXCompilationResult[];
    currentPdfPath: string | null;
    
    // Settings
    compilationDelay: number; // ms to wait before auto-compilation
    maxHistorySize: number;
    
    // State tracking
    lastSourceModified: number;
    lastCompilationTime: number;
    pendingCompilation: boolean;
    
    // Errors
    error: string | null;
}

function createLatexStore() {
    const initialState: LaTeXState = {
        isReady: false,
        mainFile: null,
        compilationStatus: 'idle',
        isCompiling: false,
        autoCompile: true,
        lastCompilation: null,
        compilationHistory: [],
        currentPdfPath: null,
        compilationDelay: 1000,
        maxHistorySize: 10,
        lastSourceModified: 0,
        lastCompilationTime: 0,
        pendingCompilation: false,
        error: null
    };

    const { subscribe, set, update } = writable<LaTeXState>(initialState);

    // Derived stores for common queries
    const hasErrors = derived([{ subscribe }], ([$latex]) => {
        return ($latex.lastCompilation?.errors?.length || 0) > 0;
    });

    const hasWarnings = derived([{ subscribe }], ([$latex]) => {
        return ($latex.lastCompilation?.warnings?.length || 0) > 0;
    });

    const compilationAge = derived([{ subscribe }], ([$latex]) => {
        if (!$latex.lastCompilation) return null;
        return Date.now() - $latex.lastCompilation.timestamp;
    });

    // Internal state
    let isInitialized = false;
    let compilationTimer: ReturnType<typeof setTimeout> | null = null;
    let workspaceUnsubscribe: (() => void) | null = null;

    const store = {
        subscribe,
        
        // Derived stores
        hasErrors,
        hasWarnings,
        compilationAge,

        // Initialization
        async initialize(): Promise<void> {
            if (!browser || isInitialized) return;

            console.log('LaTeXStore: Initializing...');

            try {
                // Subscribe to workspace changes
                workspaceUnsubscribe = workspaceStore.subscribe($workspace => {
                    store.handleWorkspaceChange($workspace);
                });

                // Listen for workspace file change events
                if (browser) {
                    window.addEventListener('workspace:file-changed', store.handleFileChangeEvent);
                }

                // Get initial state from workspace
                const workspaceState = workspaceStore.getCurrentState();
                if (workspaceState.isReady) {
                    store.handleWorkspaceChange(workspaceState);
                } else if (workspaceState.mainLatexFile) {
                    // Even if workspace isn't fully ready, set up main file if available
                    console.log(`LaTeXStore: Setting up main file during init: ${workspaceState.mainLatexFile}`);
                    update(state => ({
                        ...state,
                        mainFile: workspaceState.mainLatexFile,
                        currentPdfPath: workspaceState.mainLatexFile ? 
                            workspaceState.mainLatexFile.replace(/\.tex$/, '.pdf') : null
                    }));
                }

                update(state => ({
                    ...state,
                    isReady: true
                }));

                isInitialized = true;
                console.log('LaTeXStore: Initialized successfully');

            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : 'Failed to initialize LaTeX store';
                console.error('LaTeXStore initialization failed:', errorMessage);
                
                update(state => ({
                    ...state,
                    error: errorMessage
                }));
            }
        },

        // Workspace integration
        handleWorkspaceChange(workspaceState: any): void {
            const currentState = get({ subscribe });
            
            // Update main file if it changed
            if (workspaceState.mainLatexFile !== currentState.mainFile) {
                console.log(`LaTeXStore: Main file changed to ${workspaceState.mainLatexFile}`);
                
                update(state => ({
                    ...state,
                    mainFile: workspaceState.mainLatexFile,
                    currentPdfPath: workspaceState.mainLatexFile ? 
                        workspaceState.mainLatexFile.replace(/\.tex$/, '.pdf') : null
                }));

                // Check if we need to compile the new main file
                if (workspaceState.mainLatexFile && currentState.autoCompile) {
                    store.scheduleCompilation('main_file_changed');
                }
            }

            // Check for changes to LaTeX files
            if (workspaceState.isReady && currentState.autoCompile) {
                const latexFiles = workspaceState.latexFiles || [];
                let shouldCompile = false;
                let newestModTime = 0;

                for (const latexPath of latexFiles) {
                    const file = workspaceState.files?.get(latexPath);
                    if (file && file.lastModified > newestModTime) {
                        newestModTime = file.lastModified;
                    }
                    if (file && file.isDirty && file.lastModified > currentState.lastCompilationTime) {
                        shouldCompile = true;
                    }
                }

                if (shouldCompile && newestModTime > currentState.lastSourceModified) {
                    update(state => ({ ...state, lastSourceModified: newestModTime }));
                    store.scheduleCompilation('source_modified');
                }
            }
        },

        handleFileChangeEvent(event: Event): void {
            const customEvent = event as CustomEvent;
            const { path, changeType } = customEvent.detail;

            if (path.endsWith('.tex')) {
                console.log(`LaTeXStore: LaTeX file ${changeType}: ${path}`);
                
                const currentState = get({ subscribe });
                if (currentState.autoCompile) {
                    store.scheduleCompilation(`file_${changeType}`);
                }
            }
        },

        // Compilation management
        scheduleCompilation(reason: string = 'manual'): void {
            const currentState = get({ subscribe });
            
            if (!currentState.mainFile) {
                console.log('LaTeXStore: No main file to compile');
                return;
            }

            if (currentState.isCompiling) {
                console.log('LaTeXStore: Already compiling, marking pending');
                update(state => ({ ...state, pendingCompilation: true }));
                return;
            }

            // Clear existing timer
            if (compilationTimer) {
                clearTimeout(compilationTimer);
            }

            console.log(`LaTeXStore: Scheduling compilation (${reason}) in ${currentState.compilationDelay}ms`);
            
            update(state => ({ 
                ...state, 
                compilationStatus: 'queued',
                pendingCompilation: false
            }));

            compilationTimer = setTimeout(() => {
                store.compileCurrentFile();
            }, currentState.compilationDelay);
        },

        async compileCurrentFile(): Promise<void> {
            const currentState = get({ subscribe });
            
            if (!currentState.mainFile) {
                console.error('LaTeXStore: No main file to compile');
                return;
            }

            console.log(`LaTeXStore: Starting compilation of ${currentState.mainFile}`);
            
            const startTime = Date.now();
            
            update(state => ({
                ...state,
                compilationStatus: 'compiling',
                isCompiling: true,
                error: null
            }));

            // Emit compilation start event
            store.emitCompilationEvent('latex-compiling', { mainFile: currentState.mainFile });

            try {
                // Use platform-specific LaTeX compilation
                const result = await platformApi.compileLatex({ 
                    provider: LaTeXProvider.Auto 
                });
                const duration = Date.now() - startTime;
                
                const compilationResult: LaTeXCompilationResult = {
                    success: result.success,
                    outputFile: result.output_file,
                    pdfPath: result.output_file,
                    errors: result.errors || [],
                    warnings: [],
                    duration,
                    timestamp: Date.now()
                };

                // Update state
                update(state => {
                    const newHistory = [...state.compilationHistory, compilationResult]
                        .slice(-state.maxHistorySize);

                    return {
                        ...state,
                        compilationStatus: result.success ? 'success' : 'error',
                        isCompiling: false,
                        lastCompilation: compilationResult,
                        compilationHistory: newHistory,
                        lastCompilationTime: Date.now(),
                        currentPdfPath: result.success ? (result.output_file || null) : state.currentPdfPath,
                        error: result.success ? null : 'Compilation failed'
                    };
                });

                // Emit completion event
                if (result.success) {
                    console.log(`LaTeXStore: Compilation successful - ${result.output_file}`);
                    store.emitCompilationEvent('latex-compiled', {
                        outputFile: result.output_file,
                        message: `Compilation successful with pdflatex`
                    });
                } else {
                    console.error('LaTeXStore: Compilation failed:', compilationResult.errors);
                    store.emitCompilationEvent('latex-compile-error', {
                        errors: compilationResult.errors,
                        message: 'LaTeX compilation failed'
                    });
                }

                // Handle pending compilation
                const updatedState = get({ subscribe });
                if (updatedState.pendingCompilation) {
                    console.log('LaTeXStore: Processing pending compilation');
                    update(state => ({ ...state, pendingCompilation: false }));
                    setTimeout(() => store.scheduleCompilation('pending'), 100);
                }

            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : 'Compilation failed';
                const duration = Date.now() - startTime;
                
                console.error('LaTeXStore: Compilation error:', error);

                const failedResult: LaTeXCompilationResult = {
                    success: false,
                    errors: [errorMessage],
                    warnings: [],
                    duration,
                    timestamp: Date.now()
                };

                update(state => {
                    const newHistory = [...state.compilationHistory, failedResult]
                        .slice(-state.maxHistorySize);

                    return {
                        ...state,
                        compilationStatus: 'error',
                        isCompiling: false,
                        lastCompilation: failedResult,
                        compilationHistory: newHistory,
                        error: errorMessage
                    };
                });

                // Emit error event
                store.emitCompilationEvent('latex-compile-error', {
                    errors: [errorMessage],
                    message: errorMessage
                });
            }
        },

        // Manual compilation controls
        async compile(): Promise<void> {
            store.scheduleCompilation('manual');
        },

        async forceCompile(): Promise<void> {
            // Cancel any pending compilation and compile immediately
            if (compilationTimer) {
                clearTimeout(compilationTimer);
                compilationTimer = null;
            }
            
            await store.compileCurrentFile();
        },

        // Settings
        setAutoCompile(enabled: boolean): void {
            update(state => ({
                ...state,
                autoCompile: enabled
            }));
            console.log(`LaTeXStore: Auto-compile ${enabled ? 'enabled' : 'disabled'}`);
        },

        setCompilationDelay(delay: number): void {
            update(state => ({
                ...state,
                compilationDelay: Math.max(100, delay)
            }));
        },

        setMainFile(filePath: string | null): void {
            const currentState = get({ subscribe });
            
            if (currentState.mainFile === filePath) return;
            
            update(state => ({
                ...state,
                mainFile: filePath,
                currentPdfPath: filePath ? filePath.replace(/\.tex$/, '.pdf') : null
            }));

            if (filePath && currentState.autoCompile) {
                store.scheduleCompilation('main_file_set');
            }
        },

        // Event emission
        emitCompilationEvent(eventType: string, detail: any): void {
            // Emit to EventStore based on event type
            if (eventType === 'latex-compiling') {
                eventStore.events.compilationQueued(detail.mainFile, detail.reason);
            } else if (eventType === 'latex-compiled') {
                eventStore.events.compilationCompleted(detail.mainFile, detail.pdfPath);
            } else if (eventType === 'latex-compile-error') {
                eventStore.events.compilationFailed(detail.mainFile || 'unknown', detail.errors || [detail.message]);
            }

            // Legacy: emit browser events for backward compatibility
            if (browser) {
                const event = new CustomEvent(eventType, { detail });
                window.dispatchEvent(event);
            }
        },

        // Status queries
        canCompile(): boolean {
            const state = get({ subscribe });
            return !!(state.isReady && state.mainFile && !state.isCompiling);
        },

        getCompilationStatus(): string {
            const state = get({ subscribe });
            
            if (state.isCompiling) return 'Compiling...';
            if (state.compilationStatus === 'queued') return 'Queued for compilation';
            if (state.compilationStatus === 'success') return 'Compilation successful';
            if (state.compilationStatus === 'error') return 'Compilation failed';
            return 'Ready to compile';
        },

        getCurrentPdfPath(): string | null {
            const state = get({ subscribe });
            return state.currentPdfPath;
        },

        getLastErrors(): string[] {
            const state = get({ subscribe });
            return state.lastCompilation?.errors || [];
        },

        getCurrentState(): LaTeXState {
            return get({ subscribe });
        },

        // Agent integration - handle external compilation triggers
        handleAgentFileOperation(tool: string, path: string): void {
            if (path.endsWith('.tex')) {
                console.log(`LaTeXStore: Agent ${tool} on LaTeX file ${path}`);
                
                const currentState = get({ subscribe });
                
                // Update source modification time
                update(state => ({
                    ...state,
                    lastSourceModified: Date.now()
                }));
                
                // Only compile on final operations, not intermediate ones
                // This prevents excessive compilation during agent work
                if (currentState.autoCompile) {
                    // Only compile for create_file or for write_file operations
                    // Avoid compiling for read_file or other operations
                    if (tool === 'create_file' || tool === 'write_file') {
                        console.log(`LaTeXStore: Scheduling compilation for agent ${tool}`);
                        
                        // Use longer delay for agent operations to allow batch processing
                        const originalDelay = currentState.compilationDelay;
                        update(state => ({ ...state, compilationDelay: 2000 })); // 2 second delay
                        
                        store.scheduleCompilation(`agent_${tool}`);
                        
                        // Restore original delay after scheduling
                        setTimeout(() => {
                            update(state => ({ ...state, compilationDelay: originalDelay }));
                        }, 100);
                    } else {
                        console.log(`LaTeXStore: Skipping compilation for agent ${tool} (not a write operation)`);
                    }
                }
            }
        },

        // Cleanup
        async destroy(): Promise<void> {
            // Clear timer
            if (compilationTimer) {
                clearTimeout(compilationTimer);
                compilationTimer = null;
            }

            // Remove event listeners
            if (browser) {
                window.removeEventListener('workspace:file-changed', store.handleFileChangeEvent);
            }

            // Unsubscribe from workspace
            if (workspaceUnsubscribe) {
                workspaceUnsubscribe();
                workspaceUnsubscribe = null;
            }

            // Reset state
            set(initialState);
            isInitialized = false;
            
            console.log('LaTeXStore: Destroyed and cleaned up');
        }
    };

    return store;
}

export const latexStore = createLatexStore();