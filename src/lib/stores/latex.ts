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
    let eventUnsubscribe: (() => void) | null = null;

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

                // Subscribe to EventStore file change events
                store.subscribeToFileEvents();

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

        // Subscribe to EventStore file system events - Enhanced coordination with WorkspaceStore
        subscribeToFileEvents(): void {
            // Create event stream for LaTeX ecosystem files
            const fileSystemEvents = eventStore.createFileSystemPathStream(/\.(tex|bib|sty|cls|def|cfg|clo)$/i);
            
            // React to file changes
            const unsubscribe = fileSystemEvents.subscribe(events => {
                const latestEvent = events[events.length - 1];
                if (latestEvent) {
                    console.log(`LaTeXStore: LaTeX ecosystem file ${latestEvent.subtype}: ${latestEvent.payload.path} (source: ${latestEvent.payload.source})`);
                    
                    // Only compile on actual file modifications, not reads or other operations
                    const shouldCompile = latestEvent.subtype === 'file_modified' || 
                                        latestEvent.subtype === 'file_created';
                    
                    if (!shouldCompile) {
                        console.log(`LaTeXStore: Skipping compilation for ${latestEvent.subtype} event`);
                        return;
                    }
                    
                    const currentState = get({ subscribe });
                    if (!currentState.autoCompile) {
                        console.log('LaTeXStore: Auto-compile disabled, skipping compilation');
                        return;
                    }
                    
                    // Handle different sources appropriately
                    if (latestEvent.payload.source === 'user') {
                        // User made direct changes, compile with normal delay
                        console.log('LaTeXStore: User file change detected, scheduling compilation');
                        store.scheduleCompilation(`user_${latestEvent.subtype}`);
                    } else if (latestEvent.payload.source === 'watcher') {
                        // External file system change, compile with normal delay
                        console.log('LaTeXStore: File watcher change detected, scheduling compilation');
                        store.scheduleCompilation(`watcher_${latestEvent.subtype}`);
                    } else if (latestEvent.payload.source === 'agent') {
                        // Agent change - use longer delay and coordinate with job completion
                        console.log('LaTeXStore: Agent file change detected, scheduling delayed compilation');
                        
                        // Clear any existing timer to prevent multiple compilations
                        const timeSinceLastCompile = Date.now() - currentState.lastCompilationTime;
                        const minDelayBetweenCompiles = 3000; // 3 seconds minimum
                        
                        if (timeSinceLastCompile < minDelayBetweenCompiles) {
                            console.log(`LaTeXStore: Too soon since last compile (${timeSinceLastCompile}ms), deferring`);
                            return;
                        }
                        
                        // Use longer delay for agent operations to allow batch processing
                        const originalDelay = currentState.compilationDelay;
                        update(state => ({ ...state, compilationDelay: 2500 })); // 2.5 second delay
                        
                        store.scheduleCompilation(`agent_${latestEvent.subtype}`);
                        
                        // Restore original delay
                        setTimeout(() => {
                            update(state => ({ ...state, compilationDelay: originalDelay }));
                        }, 100);
                    }
                }
            });
            
            // Subscribe to compilation events from external sources
            const compilationEvents = eventStore.compilationEvents;
            const compUnsubscribe = compilationEvents.subscribe(events => {
                const latestEvent = events[events.length - 1];
                if (latestEvent && latestEvent.subtype === 'completed') {
                    console.log(`LaTeXStore: Compilation completed - mainFile: ${latestEvent.payload.mainFile}, pdfPath: ${latestEvent.payload.pdfPath}`);
                    
                    // Update our state to reflect the successful compilation
                    update(state => ({
                        ...state,
                        currentPdfPath: latestEvent.payload.pdfPath || state.currentPdfPath,
                        lastCompilationTime: Date.now(),
                        compilationStatus: 'success' as const
                    }));
                }
            });
            
            // Subscribe to agent events to handle job completion
            const agentEvents = eventStore.agentEvents;
            const agentUnsubscribe = agentEvents.subscribe(events => {
                const latestEvent = events[events.length - 1];
                if (latestEvent && latestEvent.subtype === 'job_complete') {
                    console.log('LaTeXStore: Agent job completed, coordinating with workspace for LaTeX compilation');
                    // Use timeout to allow workspace store to settle first
                    setTimeout(() => {
                        store.handleAgentJobComplete(latestEvent);
                    }, 500);
                }
            });
            
            // Store the unsubscribe functions for cleanup
            eventUnsubscribe = () => {
                unsubscribe();
                compUnsubscribe();
                agentUnsubscribe();
            };
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
            store.emitCompilationEvent('latex-compiling', { 
                mainFile: currentState.mainFile,
                reason: 'scheduled_compilation'
            });

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
                        mainFile: currentState.mainFile,
                        outputFile: result.output_file,
                        pdfPath: result.output_file,
                        message: `Compilation successful with pdflatex`
                    });
                } else {
                    console.error('LaTeXStore: Compilation failed:', compilationResult.errors);
                    store.emitCompilationEvent('latex-compile-error', {
                        mainFile: currentState.mainFile,
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
                const currentErrorState = get({ subscribe });
                store.emitCompilationEvent('latex-compile-error', {
                    mainFile: currentErrorState.mainFile,
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
            // Emit to EventStore - unified event system with enhanced detail
            console.log(`LaTeXStore: Emitting ${eventType}:`, detail);
            
            if (eventType === 'latex-compiling') {
                eventStore.events.compilationQueued(detail.mainFile || 'unknown', detail.reason);
            } else if (eventType === 'latex-compiled') {
                eventStore.events.compilationCompleted(detail.mainFile || 'unknown', detail.outputFile || detail.pdfPath);
            } else if (eventType === 'latex-compile-error') {
                eventStore.events.compilationFailed(detail.mainFile || 'unknown', detail.errors || [detail.message]);
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
            // Skip if not a LaTeX file
            if (!path.endsWith('.tex')) {
                return;
            }
            
            console.log(`LaTeXStore: Agent ${tool} on LaTeX file ${path}`);
            
            // Special handling for compile_latex tool
            if (tool === 'compile_latex') {
                console.log('LaTeXStore: Agent performed compilation, skipping auto-compile');
                // Update last compilation time to prevent immediate recompilation
                update(state => ({
                    ...state,
                    lastCompilationTime: Date.now(),
                    compilationStatus: 'success'
                }));
                return;
            }
            
            // Only react to write operations, not reads
            const writeOperations = ['create_file', 'write_file', 'update_file'];
            if (!writeOperations.includes(tool)) {
                console.log(`LaTeXStore: Ignoring agent ${tool} (not a write operation)`);
                return;
            }
            
            const currentState = get({ subscribe });
            
            // Update source modification time
            update(state => ({
                ...state,
                lastSourceModified: Date.now()
            }));
            
            // Only compile if auto-compile is enabled and enough time has passed
            if (currentState.autoCompile) {
                const timeSinceLastCompile = Date.now() - currentState.lastCompilationTime;
                const minDelayBetweenCompiles = 3000; // 3 seconds minimum between compilations
                
                if (timeSinceLastCompile < minDelayBetweenCompiles) {
                    console.log(`LaTeXStore: Skipping compilation (only ${timeSinceLastCompile}ms since last compile)`);
                    return;
                }
                
                console.log(`LaTeXStore: Scheduling compilation for agent ${tool}`);
                
                // Use longer delay for agent operations to allow batch processing
                const originalDelay = currentState.compilationDelay;
                update(state => ({ ...state, compilationDelay: 2000 })); // 2 second delay
                
                store.scheduleCompilation(`agent_${tool}`);
                
                // Restore original delay after scheduling
                setTimeout(() => {
                    update(state => ({ ...state, compilationDelay: originalDelay }));
                }, 100);
            }
        },

        // Handle agent job completion - Enhanced coordination with WorkspaceStore
        async handleAgentJobComplete(event: any): Promise<void> {
            console.log('LaTeXStore: Handling agent job completion for LaTeX compilation coordination');
            
            try {
                const currentState = get({ subscribe });
                
                // Check if auto-compile is enabled and we have a main file
                if (!currentState.autoCompile) {
                    console.log('LaTeXStore: Auto-compile disabled, skipping compilation');
                    return;
                }
                
                if (!currentState.mainFile) {
                    console.log('LaTeXStore: No main LaTeX file detected, skipping compilation');
                    return;
                }
                
                // Get WorkspaceStore state to check for LaTeX files
                const { workspaceStore } = await import('./workspace');
                const workspaceState = workspaceStore.getCurrentState();
                
                if (workspaceState.latexFiles.length === 0) {
                    console.log('LaTeXStore: No LaTeX files in workspace, skipping compilation');
                    return;
                }
                
                // Check if any LaTeX ecosystem files are dirty or recently modified
                const LATEX_ECOSYSTEM_PATTERN = /\.(tex|bib|sty|cls|def|cfg|clo)$/i;
                let hasLatexChanges = false;
                let latexFilesNeedingCompilation: string[] = [];
                
                // Check workspace files for recent modifications
                for (const [filePath, file] of workspaceState.files.entries()) {
                    if (LATEX_ECOSYSTEM_PATTERN.test(filePath)) {
                        // Check if file was modified recently (within last 10 seconds) or is dirty
                        const isRecentlyModified = file.lastModified > (Date.now() - 10000);
                        const isDirty = file.isDirty;
                        
                        if (isRecentlyModified || isDirty) {
                            hasLatexChanges = true;
                            latexFilesNeedingCompilation.push(filePath);
                        }
                    }
                }
                
                if (!hasLatexChanges) {
                    console.log('LaTeXStore: No recent LaTeX ecosystem file changes detected');
                    return;
                }
                
                console.log('LaTeXStore: LaTeX files needing compilation:', latexFilesNeedingCompilation);
                
                // Prevent excessive compilation attempts
                const timeSinceLastCompile = Date.now() - currentState.lastCompilationTime;
                const minDelayBetweenCompiles = 3000; // 3 seconds minimum
                
                if (timeSinceLastCompile < minDelayBetweenCompiles) {
                    console.log(`LaTeXStore: Too soon since last compile (${timeSinceLastCompile}ms), deferring compilation`);
                    
                    // Schedule for later if there are changes
                    setTimeout(() => {
                        store.handleAgentJobComplete(event);
                    }, minDelayBetweenCompiles - timeSinceLastCompile + 500);
                    return;
                }
                
                // Force reload modified LaTeX files to ensure we have latest content
                console.log('LaTeXStore: Force reloading modified LaTeX files before compilation');
                const reloadPromises = latexFilesNeedingCompilation.map(async (filePath) => {
                    try {
                        await workspaceStore.loadFile(filePath, true);
                        console.log(`LaTeXStore: Reloaded ${filePath}`);
                    } catch (error) {
                        console.warn(`LaTeXStore: Failed to reload ${filePath}:`, error);
                    }
                });
                
                // Wait for all files to reload
                await Promise.all(reloadPromises);
                
                // Schedule compilation with delay to allow workspace to settle
                console.log('LaTeXStore: Scheduling LaTeX compilation after agent job completion and file reloading');
                setTimeout(() => {
                    store.scheduleCompilation('agent_job_completed');
                }, 1000); // 1 second delay to ensure everything is settled
                
            } catch (error) {
                console.error('LaTeXStore: Error handling agent job completion:', error);
            }
        },

        // Cleanup
        async destroy(): Promise<void> {
            // Clear timer
            if (compilationTimer) {
                clearTimeout(compilationTimer);
                compilationTimer = null;
            }

            // Unsubscribe from EventStore events
            if (eventUnsubscribe) {
                eventUnsubscribe();
                eventUnsubscribe = null;
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