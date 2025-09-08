/**
 * ProjectStore - Centralized project management for desktop platform
 * Follows STATE.md reactive store architecture
 */

import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import { isTauri } from '$lib/utils/platform';
import { fileSystemApi } from '$lib/api/adapters';
import { eventStore } from './events';
import type { ProjectInfo } from '$lib/api/types';

export interface ProjectState {
    isReady: boolean;
    isLoading: boolean;
    currentProject: ProjectInfo | null;
    error: string | null;
    lastChecked: number;
}

function createProjectStore() {
    const initialState: ProjectState = {
        isReady: false,
        isLoading: false,
        currentProject: null,
        error: null,
        lastChecked: 0
    };

    const { subscribe, set, update } = writable<ProjectState>(initialState);

    let initialized = false;

    const projectStore = {
        subscribe,

        /**
         * Initialize the project store
         * Called by InitializationOrchestrator
         */
        async initialize(): Promise<void> {
            if (!browser || !isTauri()) {
                // On web, we don't need project selection
                update(state => ({
                    ...state,
                    isReady: true,
                    currentProject: { path: '', name: 'Web Mode' } as ProjectInfo
                }));
                return;
            }

            if (initialized) {
                console.log('ProjectStore: Already initialized');
                return;
            }

            console.log('ProjectStore: Initializing...');
            
            update(state => ({
                ...state,
                isLoading: true,
                error: null
            }));

            try {
                // Check if a project is already selected
                const currentProject = await fileSystemApi.getCurrentProject();
                
                update(state => ({
                    ...state,
                    isReady: true,
                    isLoading: false,
                    currentProject: currentProject || null,
                    lastChecked: Date.now()
                }));

                initialized = true;
                
                console.log('ProjectStore: Initialized with project:', currentProject?.path || 'none');
                
                // Emit event if project exists
                if (currentProject) {
                    eventStore.events.projectChanged(currentProject.path);
                }
                
            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : 'Failed to check current project';
                console.error('ProjectStore: Initialization failed:', error);
                
                update(state => ({
                    ...state,
                    isReady: true, // Still mark as ready, just with no project
                    isLoading: false,
                    currentProject: null,
                    error: errorMessage
                }));
                
                initialized = true;
            }
        },

        /**
         * Select a new project folder
         */
        async selectProject(): Promise<ProjectInfo | null> {
            if (!isTauri()) {
                throw new Error('Project selection only available on desktop');
            }

            console.log('ProjectStore: Selecting project...');
            
            update(state => ({
                ...state,
                isLoading: true,
                error: null
            }));

            try {
                const projectInfo = await fileSystemApi.selectProjectFolder();
                
                if (projectInfo) {
                    update(state => ({
                        ...state,
                        isLoading: false,
                        currentProject: projectInfo,
                        lastChecked: Date.now()
                    }));

                    // Emit project changed event
                    eventStore.events.projectChanged(projectInfo.path);
                    
                    console.log('ProjectStore: Project selected:', projectInfo.path);
                    
                    // Trigger a page reload to reinitialize with the new project
                    // This ensures all stores properly reinitialize with the new context
                    setTimeout(() => {
                        window.location.reload();
                    }, 100);
                    
                    return projectInfo;
                } else {
                    // User cancelled selection
                    update(state => ({
                        ...state,
                        isLoading: false
                    }));
                    
                    console.log('ProjectStore: Project selection cancelled');
                    return null;
                }
            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : 'Failed to select project';
                console.error('ProjectStore: Failed to select project:', error);
                
                update(state => ({
                    ...state,
                    isLoading: false,
                    error: errorMessage
                }));
                
                throw error;
            }
        },

        /**
         * Clear the current project
         */
        async clearProject(): Promise<void> {
            if (!isTauri()) {
                throw new Error('Project management only available on desktop');
            }

            console.log('ProjectStore: Clearing project...');
            
            update(state => ({
                ...state,
                isLoading: true,
                error: null
            }));

            try {
                await fileSystemApi.clearProject();
                
                update(state => ({
                    ...state,
                    isLoading: false,
                    currentProject: null,
                    lastChecked: Date.now()
                }));

                // Emit project cleared event
                eventStore.events.projectChanged('');
                
                console.log('ProjectStore: Project cleared');
                
                // Reload to show welcome screen
                setTimeout(() => {
                    window.location.reload();
                }, 100);
                
            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : 'Failed to clear project';
                console.error('ProjectStore: Failed to clear project:', error);
                
                update(state => ({
                    ...state,
                    isLoading: false,
                    error: errorMessage
                }));
                
                throw error;
            }
        },

        /**
         * Check if a project is currently selected
         */
        hasProject(): boolean {
            const state = get({ subscribe });
            return !!state.currentProject;
        },

        /**
         * Get current state (for debugging)
         */
        getCurrentState(): ProjectState {
            return get({ subscribe });
        },

        /**
         * Reset store to initial state
         */
        reset(): void {
            set(initialState);
            initialized = false;
            console.log('ProjectStore: Reset to initial state');
        }
    };

    // Create derived stores for common checks
    const hasProject = derived({ subscribe }, $state => !!$state.currentProject);
    const needsProjectSelection = derived(
        { subscribe },
        $state => isTauri() && $state.isReady && !$state.currentProject
    );
    const projectPath = derived(
        { subscribe },
        $state => $state.currentProject?.path || ''
    );

    return {
        ...projectStore,
        hasProject,
        needsProjectSelection,
        projectPath
    };
}

// Export singleton instance
export const projectStore = createProjectStore();