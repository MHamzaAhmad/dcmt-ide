// Unified File Watcher Hook
import { useQueryClient } from '@tanstack/svelte-query';
import { isTauri } from '$lib/utils/platform';
import { DesktopFileWatcher } from '../adapters/desktop/fileWatcher';
import { WebFileWatcher } from '../adapters/web/fileWebSocket';
import type { FileEventData } from '../types';
import { fileSystemKeys } from './useFileSystem';
import { writable, get } from 'svelte/store';

export type FileEventType = 'created' | 'modified' | 'deleted' | 'renamed';
export type FileEventCallback = (type: FileEventType, event: FileEventData) => void;

/**
 * Hook for listening to file system changes
 * @param callback Optional callback to handle file events
 * @param enabled Whether to enable file watching
 * @returns File watcher state and controls
 */
export function useFileWatcher(
	callback?: FileEventCallback,
	enabled: boolean = true
) {
	// Use writable stores instead of $state
	const isListening = writable(false);
	const eventCount = writable(0);
	const lastEvent = writable<{ type: FileEventType; event: FileEventData } | null>(null);
	
	const queryClient = useQueryClient();
	
	let cleanup: (() => void) | null = null;
	let fileWatcher: DesktopFileWatcher | WebFileWatcher | null = null;
	let isInitialized = false;

	function handleFileEvent(type: FileEventType, event: FileEventData) {
		console.log(`File ${type}:`, event);
		
		// Update state using stores
		eventCount.update(count => count + 1);
		lastEvent.set({ type, event });
		
		// Call user callback
		callback?.(type, event);
		
		// Invalidate relevant queries based on event type and path
		invalidateRelatedQueries(type, event);
	}

	function invalidateRelatedQueries(type: FileEventType, event: FileEventData) {
		const path = event.path;
		const pathParts = path.split('/');
		
		// Invalidate all parent directory queries in the hierarchy
		for (let i = 0; i < pathParts.length; i++) {
			const directoryPath = pathParts.slice(0, i).join('/');
			queryClient.invalidateQueries({ 
				queryKey: fileSystemKeys.directoryTree(directoryPath) 
			});
		}
		
		// Handle specific event types
		switch (type) {
			case 'created':
			case 'deleted':
				// No need to invalidate file content for these
				break;
				
			case 'modified':
				// Invalidate file content for modified files
				if (!event.metadata.is_directory) {
					queryClient.invalidateQueries({ 
						queryKey: fileSystemKeys.fileContent(path) 
					});
				}
				break;
				
			case 'renamed':
				// Remove old file from cache and invalidate new path
				if (event.metadata.old_path) {
					queryClient.removeQueries({ 
						queryKey: fileSystemKeys.fileContent(event.metadata.old_path) 
					});
				}
				
				// Invalidate both old and new parent directories
				if (event.metadata.old_path) {
					const oldParentPath = event.metadata.old_path.split('/').slice(0, -1).join('/');
					queryClient.invalidateQueries({ 
						queryKey: fileSystemKeys.directoryTree(oldParentPath) 
					});
				}
				break;
		}
	}

	function startWatching() {
		// Check current listening state without creating subscription
		const currentlyListening = get(isListening);
		
		if (!enabled || currentlyListening || fileWatcher || isInitialized) return;
		
		isInitialized = true;

		if (isTauri()) {
			// Desktop: Use Tauri file watcher
			try {
				fileWatcher = new DesktopFileWatcher();
				cleanup = fileWatcher.onAny((type: string, event: FileEventData) => {
					handleFileEvent(type as FileEventType, event);
				});
				
				isListening.set(true);
				console.log('Started desktop file watcher');
			} catch (error) {
				console.error('Failed to start desktop file watcher:', error);
			}
		} else {
			// Web: Use WebFile watcher with WebSocket
			try {
				fileWatcher = new WebFileWatcher();
				cleanup = fileWatcher.onAny((type: string, event: FileEventData) => {
					handleFileEvent(type as FileEventType, event);
				});
				
				isListening.set(true);
				console.log('Started web file watcher');
			} catch (error) {
				console.error('Failed to start web file watcher:', error);
			}
		}
	}

	function stopWatching() {
		if (cleanup) {
			cleanup();
			cleanup = null;
		}
		
		if (fileWatcher) {
			fileWatcher.destroy().catch(error => {
				console.error('Error destroying file watcher:', error);
			});
			fileWatcher = null;
		}
		
		isListening.set(false);
		isInitialized = false;
		console.log('Stopped file watcher');
	}

	// Don't auto-start, let components control initialization

	return {
		isListening,
		eventCount,
		lastEvent,
		start: startWatching,
		stop: stopWatching,
		restart: () => {
			stopWatching();
			startWatching();
		},
		destroy: stopWatching
	};
}

/**
 * Simple hook that just enables file watching with query invalidation
 * Most components will use this instead of the full useFileWatcher
 */
export function useAutoRefresh(enabled: boolean = true) {
	return useFileWatcher(undefined, enabled);
}