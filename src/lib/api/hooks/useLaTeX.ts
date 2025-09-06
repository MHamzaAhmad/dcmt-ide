// LaTeX Compilation Hooks using Svelte Query
import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { get } from 'svelte/store';
import { platformApi } from '../adapters';
import type { LaTeXCompileRequest, LaTeXCompileResponse } from '../types';
import { LaTeXProvider } from '../types';

// Query Keys
export const latexKeys = {
	all: ['latex'] as const,
	compilation: (workspacePath?: string) => [...latexKeys.all, 'compilation', workspacePath] as const,
	compileStatus: () => [...latexKeys.all, 'compileStatus'] as const,
};

/**
 * Hook to compile LaTeX documents
 */
export function useCompileLatex() {
	return createMutation({
		mutationFn: (request: LaTeXCompileRequest): Promise<LaTeXCompileResponse> => {
			const result = platformApi.compileLatex(request);
			if (!result) {
				return Promise.resolve({
					success: false,
					message: 'LaTeX compilation not supported on this platform',
					errors: ['Platform does not support LaTeX compilation']
				});
			}
			return result;
		},
		retry: 1,
	});
}

/**
 * Hook to compile LaTeX with automatic error handling
 * This is a higher-level hook that provides better UX
 */
export function useAutoCompileLatex() {
	const mutation = useCompileLatex();
	
	const compileWithDefaults = async (
		provider: LaTeXCompileRequest['provider'] = LaTeXProvider.Auto
	): Promise<LaTeXCompileResponse> => {
		return new Promise((resolve, reject) => {
			get(mutation).mutate({ provider }, {
				onSuccess: (data) => resolve(data),
				onError: (error) => reject(error)
			});
		});
	};

	// Return the mutation store itself (for reactive access with $)
	// but also add our custom method
	return {
		subscribe: mutation.subscribe,
		compileWithDefaults,
		get isCompiling() { return get(mutation).status === 'pending'; },
		get error() { return get(mutation).error; },
		get data() { return get(mutation).data; },
		reset: () => get(mutation).reset(),
	};
}

/**
 * Hook to track LaTeX compilation status across the app
 * This can be used to show global compilation status
 */
export function useLatexCompilationStatus() {
	return createQuery({
		queryKey: latexKeys.compileStatus(),
		queryFn: () => ({ isCompiling: false, lastCompileTime: null }),
		staleTime: Infinity, // This is managed manually
		refetchOnWindowFocus: false,
		refetchOnMount: false,
	});
}

/**
 * Hook to manage PDF compilation results and caching
 */
export function usePDFCompilation() {
	const queryClient = useQueryClient();
	const compileMutation = useCompileLatex();
	
	const compileAndUpdateCache = async (request: LaTeXCompileRequest) => {
		// Set compilation status
		queryClient.setQueryData(latexKeys.compileStatus(), {
			isCompiling: true,
			lastCompileTime: Date.now(),
		});
		
		try {
			const result = await new Promise<LaTeXCompileResponse>((resolve, reject) => {
				get(compileMutation).mutate(request, {
					onSuccess: (data) => resolve(data),
					onError: (error) => reject(error)
				});
			});
			
			// Update compilation status
			queryClient.setQueryData(latexKeys.compileStatus(), {
				isCompiling: false,
				lastCompileTime: Date.now(),
				lastResult: result,
			});
			
			return result;
		} catch (error) {
			// Update compilation status with error
			queryClient.setQueryData(latexKeys.compileStatus(), {
				isCompiling: false,
				lastCompileTime: Date.now(),
				lastError: error,
			});
			throw error;
		}
	};
	
	return {
		...compileMutation,
		compileAndUpdateCache,
		get isCompiling() { return get(compileMutation).status === 'pending'; },
	};
}

/**
 * Hook to invalidate LaTeX queries (useful for cleanup)
 */
export function useRefreshLatex() {
	const queryClient = useQueryClient();
	
	return () => {
		queryClient.invalidateQueries({ 
			queryKey: latexKeys.all 
		});
	};
}

/**
 * Hook to find the main LaTeX file with documentclass
 */
export function useFindMainLatexFile() {
	return createQuery({
		queryKey: [...latexKeys.all, 'findMain'],
		queryFn: async (): Promise<string> => {
			return await platformApi.findMainLatexFile();
		},
		retry: 1,
		staleTime: 5 * 60 * 1000, // 5 minutes
	});
}