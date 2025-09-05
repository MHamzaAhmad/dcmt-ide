// Unified File System Hooks using Svelte Query
import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { fileSystemApi } from '../adapters';
import type { FileInfo, FileContent } from '../types';

// Query Keys
export const fileSystemKeys = {
	all: ['fileSystem'] as const,
	directoryTree: (path: string) => [...fileSystemKeys.all, 'directoryTree', path] as const,
	fileContent: (path: string) => [...fileSystemKeys.all, 'fileContent', path] as const,
	fileExists: (path: string) => [...fileSystemKeys.all, 'fileExists', path] as const,
};

/**
 * Hook to get directory tree
 * @param path - Directory path to get tree for
 * @param enabled - Whether to enable the query
 */
export function useDirectoryTree(path: string = '', enabled: boolean = true) {
	return createQuery({
		queryKey: fileSystemKeys.directoryTree(path),
		queryFn: () => fileSystemApi.getDirectoryTree(path),
		enabled,
		staleTime: 30000, // 30 seconds
		retry: 2,
	});
}

/**
 * Hook to get file content
 * @param path - File path to read
 * @param enabled - Whether to enable the query
 */
export function useFileContent(path: string, enabled: boolean = true) {
	return createQuery({
		queryKey: fileSystemKeys.fileContent(path),
		queryFn: () => fileSystemApi.readFileContent(path),
		enabled: enabled && !!path,
		staleTime: 60000, // 1 minute
		retry: 1,
	});
}

/**
 * Hook to check if file exists
 * @param path - File path to check
 * @param enabled - Whether to enable the query
 */
export function useFileExists(path: string, enabled: boolean = true) {
	return createQuery({
		queryKey: fileSystemKeys.fileExists(path),
		queryFn: () => fileSystemApi.fileExists(path),
		enabled: enabled && !!path,
		staleTime: 30000, // 30 seconds
		retry: 1,
	});
}

/**
 * Hook to write file content
 */
export function useWriteFileContent() {
	const queryClient = useQueryClient();
	
	return createMutation({
		mutationFn: ({ path, content }: { path: string; content: string }) =>
			fileSystemApi.writeFileContent(path, content),
		onSuccess: (_, variables) => {
			// Invalidate file content query
			queryClient.invalidateQueries({ 
				queryKey: fileSystemKeys.fileContent(variables.path) 
			});
			// Invalidate all parent directory queries in the hierarchy
			const pathParts = variables.path.split('/');
			for (let i = 0; i < pathParts.length; i++) {
				const directoryPath = pathParts.slice(0, i).join('/');
				queryClient.invalidateQueries({ 
					queryKey: fileSystemKeys.directoryTree(directoryPath) 
				});
			}
		},
	});
}

/**
 * Hook to create file or directory
 */
export function useCreateFile() {
	const queryClient = useQueryClient();
	
	return createMutation({
		mutationFn: ({ 
			path, 
			content, 
			isDirectory = false 
		}: { 
			path: string; 
			content?: string; 
			isDirectory?: boolean 
		}) =>
			fileSystemApi.createFile(path, content, isDirectory),
		onSuccess: (_, variables) => {
			// Invalidate all parent directory queries in the hierarchy
			const pathParts = variables.path.split('/');
			for (let i = 0; i < pathParts.length; i++) {
				const directoryPath = pathParts.slice(0, i).join('/');
				queryClient.invalidateQueries({ 
					queryKey: fileSystemKeys.directoryTree(directoryPath) 
				});
			}
		},
	});
}

/**
 * Hook to delete file or directory
 */
export function useDeleteFile() {
	const queryClient = useQueryClient();
	
	return createMutation({
		mutationFn: (path: string) => fileSystemApi.deleteFile(path),
		onSuccess: (_, path) => {
			// Remove file content from cache
			queryClient.removeQueries({ 
				queryKey: fileSystemKeys.fileContent(path) 
			});
			// Invalidate all parent directory queries in the hierarchy
			const pathParts = path.split('/');
			for (let i = 0; i < pathParts.length; i++) {
				const directoryPath = pathParts.slice(0, i).join('/');
				queryClient.invalidateQueries({ 
					queryKey: fileSystemKeys.directoryTree(directoryPath) 
				});
			}
		},
	});
}

/**
 * Hook to rename file or directory
 */
export function useRenameFile() {
	const queryClient = useQueryClient();
	
	return createMutation({
		mutationFn: ({ oldPath, newPath }: { oldPath: string; newPath: string }) =>
			fileSystemApi.renameFile(oldPath, newPath),
		onSuccess: (_, variables) => {
			const { oldPath, newPath } = variables;
			
			// Remove old file content from cache
			queryClient.removeQueries({ 
				queryKey: fileSystemKeys.fileContent(oldPath) 
			});
			
			// Invalidate all parent directory queries for old path hierarchy
			const oldPathParts = oldPath.split('/');
			for (let i = 0; i < oldPathParts.length; i++) {
				const directoryPath = oldPathParts.slice(0, i).join('/');
				queryClient.invalidateQueries({ 
					queryKey: fileSystemKeys.directoryTree(directoryPath) 
				});
			}
			
			// Invalidate all parent directory queries for new path hierarchy
			const newPathParts = newPath.split('/');
			for (let i = 0; i < newPathParts.length; i++) {
				const directoryPath = newPathParts.slice(0, i).join('/');
				queryClient.invalidateQueries({ 
					queryKey: fileSystemKeys.directoryTree(directoryPath) 
				});
			}
		},
	});
}

/**
 * Hook to invalidate all file system queries (useful for refreshing)
 */
export function useRefreshFileSystem() {
	const queryClient = useQueryClient();
	
	return () => {
		queryClient.invalidateQueries({ 
			queryKey: fileSystemKeys.all 
		});
	};
}