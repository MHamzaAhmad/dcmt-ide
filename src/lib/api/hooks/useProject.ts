// Project Management Hooks (Desktop Only)
import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { get } from 'svelte/store';
import { fileSystemApi } from '../adapters';
import { isTauri } from '$lib/utils/platform';
import type { ProjectInfo } from '../types';

// Query Keys
export const projectKeys = {
	all: ['project'] as const,
	current: () => [...projectKeys.all, 'current'] as const,
	info: () => [...projectKeys.all, 'info'] as const,
};

/**
 * Hook to get current project information
 * Only available on desktop (Tauri) platform
 */
export function useCurrentProject(enabled: boolean = true) {
	return createQuery({
		queryKey: projectKeys.current(),
		queryFn: async (): Promise<ProjectInfo | null> => {
			if (!isTauri()) {
				return null;
			}
			return await fileSystemApi.getCurrentProject() || null;
		},
		enabled: enabled && isTauri(),
		staleTime: 30000, // 30 seconds
		retry: 1,
	});
}

/**
 * Hook to select a project folder (desktop only)
 */
export function useSelectProject() {
	const queryClient = useQueryClient();
	
	return createMutation({
		mutationFn: async (): Promise<ProjectInfo | null> => {
			if (!isTauri()) {
				throw new Error('Project selection only available on desktop');
			}
			
			const result = await fileSystemApi.selectProjectFolder();
			return result || null;
		},
		onSuccess: (projectInfo) => {
			console.log('useSelectProject onSuccess called with:', projectInfo);
			
			// Update current project cache
			queryClient.setQueryData(projectKeys.current(), projectInfo);
			console.log('Updated query cache for current project');
			
			// Invalidate file system queries to refresh with new workspace
			queryClient.invalidateQueries({ queryKey: ['fileSystem'] });
			console.log('Invalidated file system queries');
			
			// Emit custom event for other components to react
			if (typeof window !== 'undefined' && projectInfo) {
				window.dispatchEvent(
					new CustomEvent('project-selected', { 
						detail: projectInfo 
					})
				);
				console.log('Emitted project-selected event');
			}
		},
		onError: (error) => {
			console.error('Failed to select project:', error);
		},
	});
}

/**
 * Hook to clear current project (desktop only)
 */
export function useClearProject() {
	const queryClient = useQueryClient();
	
	return createMutation({
		mutationFn: async (): Promise<void> => {
			if (!isTauri()) {
				throw new Error('Project management only available on desktop');
			}
			
			await fileSystemApi.clearProject();
		},
		onSuccess: () => {
			// Clear current project from cache
			queryClient.setQueryData(projectKeys.current(), null);
			
			// Clear all file system queries
			queryClient.removeQueries({ queryKey: ['fileSystem'] });
			
			// Emit custom event
			if (typeof window !== 'undefined') {
				window.dispatchEvent(
					new CustomEvent('project-cleared')
				);
			}
		},
		onError: (error) => {
			console.error('Failed to clear project:', error);
		},
	});
}

/**
 * Hook to check if a project is currently selected
 */
export function useHasProject() {
	const currentProjectQuery = useCurrentProject();
	
	// Since TanStack Query returns a readable store, we can access values using get()
	return {
		get hasProject() { return !!get(currentProjectQuery).data; },
		get project() { return get(currentProjectQuery).data; },
		get isLoading() { return get(currentProjectQuery).isLoading; },
		supportsProjects: isTauri(),
	};
}

/**
 * Custom hook that combines project status with file system readiness
 */
export function useWorkspaceReady() {
	const currentProjectQuery = useCurrentProject();
	
	return () => {
		const projectData = get(currentProjectQuery);
		const hasProject = !!projectData.data;
		const isLoading = projectData.isLoading;
		const supportsProjects = isTauri();
		
		// For web, workspace is always ready (uses predefined /workspace)
		// For desktop, workspace is ready when project is selected
		const isReady = supportsProjects ? hasProject : true;
		const needsSetup = supportsProjects && !hasProject && !isLoading;
		
		console.log('useWorkspaceReady state:', {
			hasProject,
			isLoading,
			isReady,
			needsSetup,
			supportsProjects,
			projectData: projectData.data
		});
		
		return {
			isReady,
			isLoading,
			needsSetup,
			hasProject,
			supportsProjects,
		};
	};
}