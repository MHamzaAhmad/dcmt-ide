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
import { gitStore } from './git';
import { eventStore } from './events';
import { projectStore } from './project';
import { authStore } from './auth';
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
            name: 'auth',
            description: 'Check authentication',
            status: 'pending'
        },
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
            name: 'git',
            description: 'Initialize Git version control',
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

            // Filter steps based on platform and options
            let steps = [...initialSteps];

            // Remove auth step for desktop
            if (isTauri()) {
                steps = steps.filter(step => step.name !== 'auth');
            }

            // Remove agent step if requested
            if (skipAgentInit) {
                steps = steps.filter(step => step.name !== 'agent');
            }

            update(state => ({
                ...initialState,
                isInitializing: true,
                steps
            }));

            try {
                // // Step 0: Check authentication (web only)
                if (!isTauri()) {
                    await orchestrator.executeStep('auth', async () => {
                        console.log('InitializationOrchestrator: Checking authentication...');
                        await authStore.initialize();

                        // Check if authenticated
                        const authState = authStore.getCurrentState();
                        if (!authState.isAuthenticated) {
                            console.log('InitializationOrchestrator: User not authenticated, redirecting...');
                            // Auth store will handle redirect, just stop initialization
                            throw new Error('Authentication required');
                        }

                        console.log('InitializationOrchestrator: User authenticated');

                        // Emit auth event
                        if (authState.user) {
                            eventStore.events.authAuthenticated(authState.user.id || 'unknown', authState.user.email);
                        }
                    });
                }

                // Step 1: Check project selection (desktop only)
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

                // Step 2: Initialize EventStore (quick, synchronous setup)
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

                // Step 3: Initialize Workspace (required for other steps)
                await orchestrator.executeStep('workspace', async () => {
                    console.log('InitializationOrchestrator: Initializing workspace...');
                    await workspaceStore.initialize(rootPath, queryClient);
                    
                    // Verify workspace is ready
                    const workspaceState = workspaceStore.getCurrentState();
                    if (!workspaceState.isReady) {
                        throw new Error('Workspace initialization incomplete');
                    }
                });

                // Step 4-7: Initialize LaTeX, PDF, Git, and Agent in parallel (independent operations)
                const parallelInitPromises: Promise<void>[] = [];
                
                // LaTeX initialization
                parallelInitPromises.push(
                    orchestrator.executeStep('latex', async () => {
                        console.log('InitializationOrchestrator: Initializing LaTeX system...');
                        await latexStore.initialize();
                        
                        // Verify LaTeX is ready
                        const latexState = latexStore.getCurrentState();
                        if (!latexState.isReady) {
                            throw new Error('LaTeX system initialization incomplete');
                        }
                        
                        // Trigger startup compilation and PDF loading
                        console.log('InitializationOrchestrator: Triggering startup compilation...');
                        try {
                            await latexStore.triggerStartupCompilation();
                        } catch (error) {
                            // Don't fail initialization if startup compilation fails
                            console.warn('InitializationOrchestrator: Startup compilation failed:', error);
                        }
                    })
                );

                // PDF initialization (can be parallel with LaTeX as they're independent)
                parallelInitPromises.push(
                    orchestrator.executeStep('pdf', async () => {
                        console.log('InitializationOrchestrator: Initializing PDF system...');
                        
                        // Use lazy loading for PDF.js to improve startup performance
                        try {
                            await pdfStore.initialize();
                            
                            // Verify PDF is ready
                            const pdfState = pdfStore.getCurrentState();
                            if (!pdfState.isReady) {
                                throw new Error('PDF system initialization incomplete');
                            }
                        } catch (error) {
                            // Don't fail the entire initialization if PDF fails
                            // User can still use the app without PDF preview
                            console.warn('InitializationOrchestrator: PDF initialization failed, enabling graceful degradation:', error);
                            
                            // Enable graceful degradation following EventStore pattern
                            try {
                                pdfStore.enableGracefulDegradation();
                                
                                // Mark step as completed with warning rather than failed
                                // This allows the app to continue functioning
                                update(state => ({
                                    ...state,
                                    steps: state.steps.map(step => 
                                        step.name === 'pdf' 
                                            ? { ...step, status: 'completed', error: `Warning: ${error instanceof Error ? error.message : 'PDF initialization failed'} - graceful degradation enabled` }
                                            : step
                                    )
                                }));
                            } catch (degradationError) {
                                // If even graceful degradation fails, mark as failed
                                console.error('InitializationOrchestrator: PDF graceful degradation failed:', degradationError);
                                update(state => ({
                                    ...state,
                                    steps: state.steps.map(step => 
                                        step.name === 'pdf' 
                                            ? { ...step, status: 'failed', error: error instanceof Error ? error.message : 'PDF initialization completely failed' }
                                            : step
                                    )
                                }));
                            }
                        }
                    })
                );

                // Git initialization (independent of LaTeX/PDF)
                parallelInitPromises.push(
                    orchestrator.executeStep('git', async () => {
                        console.log('InitializationOrchestrator: Initializing Git system...');
                        
                        try {
                            // Get current project path for desktop or empty string for web
                            let workspacePath = '';
                            if (isTauri()) {
                                const projectState = projectStore.getCurrentState();
                                workspacePath = projectState.currentProject?.path || '';
                            }
                            
                            await gitStore.initialize(workspacePath);
                            
                            // Verify Git is ready
                            const gitState = gitStore.getCurrentState();
                            if (!gitState.isReady) {
                                throw new Error('Git system initialization incomplete');
                            }
                        } catch (error) {
                            // Don't fail the entire initialization if Git fails
                            // User can still use the app without version control
                            console.warn('InitializationOrchestrator: Git initialization failed, continuing without version control:', error);
                            
                            // Mark git as failed but don't throw
                            update(state => ({
                                ...state,
                                steps: state.steps.map(step => 
                                    step.name === 'git' 
                                        ? { ...step, status: 'failed', error: error instanceof Error ? error.message : 'Git initialization failed' }
                                        : step
                                )
                            }));
                        }
                    })
                );

                // Agent initialization (independent of LaTeX/PDF/Git)
                if (!skipAgentInit) {
                    parallelInitPromises.push(
                        orchestrator.executeStep('agent', async () => {
                            console.log('InitializationOrchestrator: Initializing agent system...');
                            
                            // Set query client if available
                            if (queryClient) {
                                agentStore.setQueryClient(queryClient);
                            }
                            
                            try {
                                await agentStore.initialize();
                            } catch (error) {
                                // Don't fail the entire initialization if agent fails
                                // User can still use the app without agent functionality
                                console.warn('InitializationOrchestrator: Agent initialization failed, continuing without agent features:', error);
                                
                                // Mark agent as failed but don't throw
                                update(state => ({
                                    ...state,
                                    steps: state.steps.map(step => 
                                        step.name === 'agent' 
                                            ? { ...step, status: 'failed', error: error instanceof Error ? error.message : 'Agent initialization failed' }
                                            : step
                                    )
                                }));
                            }
                        })
                    );
                }

                // Wait for all parallel operations to complete
                await Promise.all(parallelInitPromises);

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
                await gitStore.cleanup();
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

        async initializeGit(workspacePath: string = ''): Promise<void> {
            const gitState = gitStore.getCurrentState();
            if (!gitState.isReady) {
                await gitStore.initialize(workspacePath);
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