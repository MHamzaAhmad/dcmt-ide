/**
 * LaTeXStore - Simple LaTeX compilation state management
 * Subscribes to backend compilation events and maintains compilation status
 */

import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';
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
    
    // Results
    lastCompilation: LaTeXCompilationResult | null;
    currentPdfPath: string | null;
    
    // Settings
    autoCompile: boolean;
    
    // Errors
    error: string | null;
    
    // Error Panel UI State
    showErrorPanel: boolean;
}

function createLatexStore() {
    const initialState: LaTeXState = {
        isReady: false,
        mainFile: null,
        compilationStatus: 'idle',
        autoCompile: true,
        lastCompilation: null,
        currentPdfPath: null,
        error: null,
        showErrorPanel: true
    };

    const { subscribe, set, update } = writable<LaTeXState>(initialState);

    // Derived stores for common queries
    const hasErrors = derived([{ subscribe }], ([$latex]) => {
        return ($latex.lastCompilation?.errors?.length || 0) > 0;
    });

    const hasWarnings = derived([{ subscribe }], ([$latex]) => {
        return ($latex.lastCompilation?.warnings?.length || 0) > 0;
    });

    // Internal state
    let isInitialized = false;
    let compilationEventUnsubscribe: (() => void) | null = null;

    const store = {
        subscribe,
        
        // Derived stores
        hasErrors,
        hasWarnings,

        // Initialization
        async initialize(): Promise<void> {
            if (!browser || isInitialized) return;

            console.log('LaTeXStore: Initializing simplified store...');

            try {
                // Subscribe to backend compilation events
                compilationEventUnsubscribe = platformApi.onCompilationEvent((event) => {
                    store.handleCompilationEvent(event);
                });

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

        // Handle compilation events from backend
        handleCompilationEvent(event: any): void {
            console.log('LaTeXStore: Received compilation event:', event);

            switch (event.event_type) {
                case 'main_file_detected':
                    update(state => ({
                        ...state,
                        mainFile: event.main_file,
                        currentPdfPath: event.main_file ? event.main_file.replace(/\.tex$/, '.pdf') : null
                    }));
                    break;

                case 'queued':
                    update(state => ({
                        ...state,
                        compilationStatus: 'queued',
                        error: null
                    }));
                    
                    // Emit to EventStore
                    eventStore.events.compilationQueued(event.main_file, event.metadata?.reason);
                    break;

                case 'started':
                    update(state => ({
                        ...state,
                        compilationStatus: 'compiling',
                        error: null
                    }));
                    
                    // Note: EventStore doesn't have compilationStarted, just update state
                    break;

                case 'success':
                    const successResult: LaTeXCompilationResult = {
                        success: true,
                        outputFile: event.metadata?.pdf_path,
                        pdfPath: event.metadata?.pdf_path,
                        errors: [],
                        warnings: [],
                        duration: event.metadata?.duration_ms || 0,
                        timestamp: event.timestamp
                    };

                    update(state => ({
                        ...state,
                        compilationStatus: 'success',
                        lastCompilation: successResult,
                        currentPdfPath: event.metadata?.pdf_path || state.currentPdfPath,
                        error: null
                    }));
                    
                    // Emit to EventStore - this will trigger PDF reload!
                    if (event.metadata?.pdf_path) {
                        eventStore.events.compilationCompleted(event.main_file, event.metadata.pdf_path);
                    }
                    break;

                case 'error':
                    const errorResult: LaTeXCompilationResult = {
                        success: false,
                        errors: event.metadata?.errors || ['Compilation failed'],
                        warnings: [],
                        duration: event.metadata?.duration_ms || 0,
                        timestamp: event.timestamp
                    };

                    update(state => ({
                        ...state,
                        compilationStatus: 'error',
                        lastCompilation: errorResult,
                        error: 'Compilation failed'
                    }));
                    
                    // Emit to EventStore
                    eventStore.events.compilationFailed(
                        event.main_file, 
                        event.metadata?.errors || ['Compilation failed']
                    );
                    break;
            }
        },

        // Manual compilation controls
        async compile(): Promise<void> {
            try {
                await platformApi.compileLatex({ 
                    provider: LaTeXProvider.Auto 
                });
            } catch (error) {
                console.error('LaTeXStore: Manual compilation error:', error);
                update(state => ({
                    ...state,
                    error: error instanceof Error ? error.message : 'Compilation failed'
                }));
            }
        },

        async forceCompile(): Promise<void> {
            await store.compile();
        },

        // Settings
        setAutoCompile(enabled: boolean): void {
            update(state => ({
                ...state,
                autoCompile: enabled
            }));
            console.log(`LaTeXStore: Auto-compile ${enabled ? 'enabled' : 'disabled'} (frontend setting only)`);
            console.log('LaTeXStore: Backend auto-compilation is always enabled and handles file watching');
            
            // Note: Backend auto-compilation is always enabled and handles file watching automatically
            // This frontend setting is just for UI state
        },

        setMainFile(filePath: string | null): void {
            update(state => ({
                ...state,
                mainFile: filePath,
                currentPdfPath: filePath ? filePath.replace(/\.tex$/, '.pdf') : null
            }));

            console.log('LaTeXStore: Main file updated in frontend state (backend detects main file automatically)');
            
            // Note: Backend automatically detects and sets the main file
            // This is just for updating the frontend UI state
        },

        // Status queries
        canCompile(): boolean {
            const state = store.getCurrentState();
            return !!(state.isReady && state.mainFile && state.compilationStatus !== 'compiling');
        },

        getCompilationStatus(): string {
            const state = store.getCurrentState();
            
            if (state.compilationStatus === 'compiling') return 'Compiling...';
            if (state.compilationStatus === 'queued') return 'Queued for compilation';
            if (state.compilationStatus === 'success') return 'Compilation successful';
            if (state.compilationStatus === 'error') return 'Compilation failed';
            return 'Ready to compile';
        },

        getCurrentPdfPath(): string | null {
            const state = store.getCurrentState();
            return state.currentPdfPath;
        },

        getLastErrors(): string[] {
            const state = store.getCurrentState();
            return state.lastCompilation?.errors || [];
        },

        // Error Panel Methods
        showErrorPanel(): void {
            update(state => ({
                ...state,
                showErrorPanel: true
            }));
        },

        hideErrorPanel(): void {
            update(state => ({
                ...state,
                showErrorPanel: false
            }));
        },

        toggleErrorPanel(): void {
            update(state => ({
                ...state,
                showErrorPanel: !state.showErrorPanel
            }));
        },

        getRawErrors(): string {
            const state = store.getCurrentState();
            return state.lastCompilation?.errors?.join('\n') || '';
        },

        // Startup compilation
        async triggerStartupCompilation(): Promise<void> {
            const state = store.getCurrentState();
            
            if (!state.isReady) {
                console.warn('LaTeXStore: Cannot trigger startup compilation - store not ready');
                return;
            }

            try {
                console.log('LaTeXStore: Triggering startup compilation...');
                
                // Just trigger a compilation - backend will automatically:
                // 1. Detect main LaTeX file
                // 2. Compile the project
                // 3. Set up file watching for future changes
                await store.compile();
                
                console.log('LaTeXStore: Startup compilation triggered');
                
            } catch (error) {
                console.error('LaTeXStore: Startup compilation failed:', error);
                // Don't throw - startup should continue even if compilation fails
            }
        },

        getCurrentState(): LaTeXState {
            let currentState: LaTeXState;
            subscribe(state => currentState = state)();
            return currentState!;
        },

        // Cleanup
        async destroy(): Promise<void> {
            // Unsubscribe from compilation events
            if (compilationEventUnsubscribe) {
                compilationEventUnsubscribe();
                compilationEventUnsubscribe = null;
            }

            // Reset state
            set(initialState);
            isInitialized = false;
            
            console.log('LaTeXStore: Destroyed and cleaned up');
        }
    };

    // Add derived properties
    return {
        ...store,
        isCompiling: derived(store, state => state.compilationStatus === 'compiling')
    };
}

const _latexStore = createLatexStore();
export const latexStore = _latexStore;
export const isCompiling = _latexStore.isCompiling;