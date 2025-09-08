/**
 * InitializationOrchestrator - Manages predictable startup sequence for all stores
 * Ensures proper initialization order and dependency management
 */

import { writable, get } from 'svelte/store';
import { browser } from '$app/environment';
import { workspaceStore } from './workspace';
import { latexStore } from './latex';
import { pdfStore } from './pdf';
import { agentStore } from './agent';
import { eventStore } from './events';
import { projectStore } from './project';
import { isTauri } from '$lib/utils/platform';
import type { QueryClient } from '@tanstack/svelte-query';

export interface InitializationStep {
    name: string;
    description: string;
    status: 'pending' | 'running' | 'completed' | 'failed';
    duration?: number;
    error?: string;
    startTime?: number;
}

export interface InitializationState {
    isInitializing: boolean;
    isReady: boolean;
    currentStep: number;
    steps: InitializationStep[];
    totalDuration: number;
    error: string | null;
    retryCount: number;
    maxRetries: number;
}

export interface InitializationOptions {
    rootPath?: string;
    queryClient?: QueryClient;
    autoStart?: boolean;
    skipAgentInit?: boolean;
    retryOnFailure?: boolean;
}

function createInitializationOrchestrator() {
    const initialSteps: InitializationStep[] = [
        {
            name: 'project',
            description: 'Check project selection',
            status: 'pending'
        },
        {
            name: 'eventStore',
            description: 'Initialize unified event system',
            status: 'pending'
        },
        {
            name: 'workspace',
            description: 'Initialize workspace and detect files',
            status: 'pending'
        },
        {
            name: 'latex',
            description: 'Set up LaTeX compilation system',
            status: 'pending'
        },
        {
            name: 'pdf',
            description: 'Initialize PDF preview system',
            status: 'pending'
        },
        {
            name: 'agent',
            description: 'Connect to agent system',
            status: 'pending'
        }
    ];

    const initialState: InitializationState = {
        isInitializing: false,
        isReady: false,
        currentStep: -1,
        steps: [...initialSteps],
        totalDuration: 0,
        error: null,
        retryCount: 0,
        maxRetries: 2
    };

    const { subscribe, set, update } = writable<InitializationState>(initialState);

    // Internal state
    let startTime = 0;
    let isInitialized = false;

    const orchestrator = {
        subscribe,

        // Main initialization method
        async initialize(options: InitializationOptions = {}): Promise<void> {
            if (!browser) {
                console.log('InitializationOrchestrator: Not in browser, skipping initialization');
                return;
            }

            if (isInitialized) {
                console.log('InitializationOrchestrator: Already initialized');
                return;
            }

            const {
                rootPath = '',
                queryClient,
                skipAgentInit = false,
                retryOnFailure = true
            } = options;

            console.log('InitializationOrchestrator: Starting initialization sequence...');

            // Reset state
            startTime = Date.now();
            update(state => ({
                ...initialState,
                isInitializing: true,
                steps: skipAgentInit ? 
                    initialSteps.filter(step => step.name !== 'agent') : 
                    [...initialSteps]
            }));

            try {
                // Step 0: Check project selection (desktop only)
                if (isTauri()) {
                    await orchestrator.executeStep('project', async () => {
                        console.log('InitializationOrchestrator: Checking project selection...');
                        await projectStore.initialize();
                        
                        // Check if project is selected
                        const projectState = projectStore.getCurrentState();
                        if (!projectState.currentProject) {
                            console.log('InitializationOrchestrator: No project selected, stopping initialization');
                            // Don't throw error, just stop here
                            // The UI will show the welcome screen
                            return;
                        }
                        
                        console.log('InitializationOrchestrator: Project found:', projectState.currentProject.path);
                    });
                    
                    // If no project, stop initialization here
                    const projectState = projectStore.getCurrentState();
                    if (isTauri() && !projectState.currentProject) {
                        update(state => ({
                            ...state,
                            isInitializing: false,
                            isReady: false, // Not ready without a project
                            totalDuration: Date.now() - startTime
                        }));
                        console.log('InitializationOrchestrator: Stopped - awaiting project selection');
                        return;
                    }
                }

                // Step 1: Initialize EventStore
                await orchestrator.executeStep('eventStore', async () => {
                    console.log('InitializationOrchestrator: Initializing EventStore...');
                    
                    // Set up EventStore with QueryClient for query invalidation
                    if (queryClient) {
                        eventStore.setQueryClient(queryClient);
                    }
                    
                    // Enable debug logging in development
                    if (typeof window !== 'undefined' && window.location.hostname === 'localhost') {
                        eventStore.setDebugLogging(true);
                    }
                    
                    console.log('InitializationOrchestrator: EventStore initialized');
                });

                // Step 1: Initialize Workspace
                await orchestrator.executeStep('workspace', async () => {
                    console.log('InitializationOrchestrator: Initializing workspace...');
                    await workspaceStore.initialize(rootPath, queryClient);
                    
                    // Verify workspace is ready
                    const workspaceState = workspaceStore.getCurrentState();
                    if (!workspaceState.isReady) {
                        throw new Error('Workspace initialization incomplete');
                    }
                });

                // Step 2: Initialize LaTeX
                await orchestrator.executeStep('latex', async () => {
                    console.log('InitializationOrchestrator: Initializing LaTeX system...');
                    await latexStore.initialize();
                    
                    // Verify LaTeX is ready
                    const latexState = latexStore.getCurrentState();
                    if (!latexState.isReady) {
                        throw new Error('LaTeX system initialization incomplete');
                    }
                });

                // Step 3: Initialize PDF
                await orchestrator.executeStep('pdf', async () => {
                    console.log('InitializationOrchestrator: Initializing PDF system...');
                    await pdfStore.initialize();
                    
                    // Verify PDF is ready
                    const pdfState = pdfStore.getCurrentState();
                    if (!pdfState.isReady) {
                        throw new Error('PDF system initialization incomplete');
                    }
                });

                // Step 4: Initialize Agent (if not skipped)
                if (!skipAgentInit) {
                    await orchestrator.executeStep('agent', async () => {
                        console.log('InitializationOrchestrator: Initializing agent system...');
                        
                        // Set query client if available
                        if (queryClient) {
                            agentStore.setQueryClient(queryClient);
                        }
                        
                        await agentStore.initialize();
                    });
                }

                // All steps completed successfully
                const totalDuration = Date.now() - startTime;
                
                update(state => ({
                    ...state,
                    isInitializing: false,
                    isReady: true,
                    totalDuration,
                    currentStep: -1
                }));

                isInitialized = true;
                console.log(`InitializationOrchestrator: All systems initialized successfully in ${totalDuration}ms`);

                // Emit ready event through EventStore
                if (browser && eventStore) {
                    eventStore.events.systemReady(totalDuration);
                }

            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : 'Initialization failed';
                console.error('InitializationOrchestrator: Initialization failed:', error);

                update(state => ({
                    ...state,
                    isInitializing: false,
                    error: errorMessage,
                    currentStep: -1
                }));

                // Handle retry logic
                const currentState = get({ subscribe });
                if (retryOnFailure && currentState.retryCount < currentState.maxRetries) {
                    console.log(`InitializationOrchestrator: Retrying initialization (attempt ${currentState.retryCount + 1})`);
                    
                    update(state => ({
                        ...state,
                        retryCount: state.retryCount + 1
                    }));

                    // Retry after a delay
                    setTimeout(() => {
                        orchestrator.retry(options);
                    }, 2000 * (currentState.retryCount + 1));
                } else {
                    // Emit error event through EventStore
                    if (browser && eventStore) {
                        eventStore.events.systemError(errorMessage);
                    }
                    throw error;
                }
            }
        },

        // Execute a single initialization step
        async executeStep(stepName: string, stepFunction: () => Promise<void>): Promise<void> {
            const currentState = get({ subscribe });
            const stepIndex = currentState.steps.findIndex(step => step.name === stepName);
            
            if (stepIndex === -1) {
                throw new Error(`Unknown initialization step: ${stepName}`);
            }

            // Mark step as running
            update(state => ({
                ...state,
                currentStep: stepIndex,
                steps: state.steps.map((step, index) => 
                    index === stepIndex 
                        ? { ...step, status: 'running', startTime: Date.now() }
                        : step
                )
            }));

            try {
                await stepFunction();
                
                // Mark step as completed
                const stepDuration = Date.now() - (currentState.steps[stepIndex].startTime || Date.now());
                
                update(state => ({
                    ...state,
                    steps: state.steps.map((step, index) => 
                        index === stepIndex 
                            ? { ...step, status: 'completed', duration: stepDuration }
                            : step
                    )
                }));

                console.log(`InitializationOrchestrator: Step '${stepName}' completed in ${stepDuration}ms`);

            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : `Step ${stepName} failed`;
                
                // Mark step as failed
                update(state => ({
                    ...state,
                    steps: state.steps.map((step, index) => 
                        index === stepIndex 
                            ? { ...step, status: 'failed', error: errorMessage }
                            : step
                    )
                }));

                console.error(`InitializationOrchestrator: Step '${stepName}' failed:`, error);
                throw error;
            }
        },

        // Retry initialization
        async retry(options: InitializationOptions = {}): Promise<void> {
            console.log('InitializationOrchestrator: Retrying initialization...');
            
            // Reset initialization state
            isInitialized = false;
            
            // Reset all stores
            await orchestrator.reset();
            
            // Retry initialization
            await orchestrator.initialize(options);
        },

        // Reset all systems
        async reset(): Promise<void> {
            console.log('InitializationOrchestrator: Resetting all systems...');
            
            try {
                // Reset stores in reverse order
                await agentStore.destroy();
                await pdfStore.destroy();
                await latexStore.destroy();
                await workspaceStore.destroy();
                
                console.log('InitializationOrchestrator: All systems reset successfully');
                
            } catch (error) {
                console.error('InitializationOrchestrator: Error during reset:', error);
            }
        },

        // Status queries
        getInitializationProgress(): number {
            const state = get({ subscribe });
            const completedSteps = state.steps.filter(step => step.status === 'completed').length;
            return Math.round((completedSteps / state.steps.length) * 100);
        },

        getCurrentStep(): InitializationStep | null {
            const state = get({ subscribe });
            return state.currentStep >= 0 ? state.steps[state.currentStep] : null;
        },

        isSystemReady(): boolean {
            const state = get({ subscribe });
            return state.isReady;
        },

        hasErrors(): boolean {
            const state = get({ subscribe });
            return !!(state.error || state.steps.some(step => step.status === 'failed'));
        },

        getErrors(): string[] {
            const state = get({ subscribe });
            const errors: string[] = [];
            
            if (state.error) {
                errors.push(state.error);
            }
            
            state.steps.forEach(step => {
                if (step.error) {
                    errors.push(`${step.name}: ${step.error}`);
                }
            });
            
            return errors;
        },

        // Manual system management
        async initializeWorkspace(rootPath: string = '', queryClient?: QueryClient): Promise<void> {
            if (!workspaceStore.getCurrentState().isReady) {
                await workspaceStore.initialize(rootPath, queryClient);
            }
        },

        async initializeLatex(): Promise<void> {
            if (!latexStore.getCurrentState().isReady) {
                await latexStore.initialize();
            }
        },

        async initializePdf(): Promise<void> {
            if (!pdfStore.getCurrentState().isReady) {
                await pdfStore.initialize();
            }
        },

        async initializeAgent(queryClient?: QueryClient): Promise<void> {
            const agentState = agentStore.getCurrentState();
            if (!agentState.isConnected) {
                if (queryClient) {
                    agentStore.setQueryClient(queryClient);
                }
                await agentStore.initialize();
            }
        },

        // Get current state
        getCurrentState(): InitializationState {
            return get({ subscribe });
        },

        // Cleanup
        async destroy(): Promise<void> {
            console.log('InitializationOrchestrator: Shutting down...');
            
            await orchestrator.reset();
            
            set(initialState);
            isInitialized = false;
            
            console.log('InitializationOrchestrator: Shutdown complete');
        }
    };

    return orchestrator;
}

export const initializationOrchestrator = createInitializationOrchestrator();

// Convenience export for components
export { initializationOrchestrator as orchestrator };