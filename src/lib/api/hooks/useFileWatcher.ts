// Unified File Watcher Hook
import { useQueryClient, type QueryClient } from '@tanstack/svelte-query';
import { isTauri } from '$lib/utils/platform';
import { DesktopFileWatcher } from '../adapters/desktop/fileWatcher';
import { WebFileWatcher } from '../adapters/web/fileWebSocket';
import type { FileEventData } from '../types';
import { fileSystemKeys } from './useFileSystem';
import { writable, get, derived } from 'svelte/store';
import { eventStore } from '$lib/stores/events';

export type FileEventType = 'created' | 'modified' | 'deleted' | 'renamed';
export type FileEventCallback = (type: FileEventType, event: FileEventData) => void;

/**
 * Hook for listening to file system changes
 * @param callback Optional callback to handle file events
 * @param enabled Whether to enable file watching
 * @param queryClient Optional QueryClient instance
 * @returns File watcher state and controls
 */
export function useFileWatcher(
	callback?: FileEventCallback,
	enabled: boolean = true,
	queryClient?: QueryClient
) {
	// Use writable stores instead of $state
	const isListening = writable(false);
	const eventCount = writable(0);
	const lastEvent = writable<{ type: FileEventType; event: FileEventData } | null>(null);
	
	// Try to get queryClient from context if not provided, but don't fail if not available
	let queryClientInstance: QueryClient | undefined = queryClient;
	try {
		if (!queryClientInstance) {
			queryClientInstance = useQueryClient();
		}
	} catch (error) {
		// Not in component context, that's okay if queryClient was provided
		console.debug('useFileWatcher: Unable to get queryClient from context, query invalidation will be skipped');
	}
	
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
		if (!queryClientInstance) return; // Skip if no queryClient available
		
		const path = event.path;
		const pathParts = path.split('/');
		
		// Invalidate all parent directory queries in the hierarchy
		for (let i = 0; i < pathParts.length; i++) {
			const directoryPath = pathParts.slice(0, i).join('/');
			queryClientInstance.invalidateQueries({ 
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
					queryClientInstance.invalidateQueries({ 
						queryKey: fileSystemKeys.fileContent(path) 
					});
				}
				break;
				
			case 'renamed':
				// Remove old file from cache and invalidate new path
				if (event.metadata.old_path) {
					queryClientInstance.removeQueries({ 
						queryKey: fileSystemKeys.fileContent(event.metadata.old_path) 
					});
				}
				
				// Invalidate both old and new parent directories
				if (event.metadata.old_path) {
					const oldParentPath = event.metadata.old_path.split('/').slice(0, -1).join('/');
					queryClientInstance.invalidateQueries({ 
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
export function useAutoRefresh(enabled: boolean = true, queryClient?: QueryClient) {
	return useFileWatcher(undefined, enabled, queryClient);
}

/**
 * Modern file watcher hook that uses EventStore directly
 * This is the preferred way to watch for file changes
 */
export function useFileWatcherEvents(
	callback?: (event: any) => void,
	pathPattern?: string | RegExp,
	enabled: boolean = true
) {
	// Subscribe to filesystem events from EventStore
	const fileSystemEvents = pathPattern 
		? eventStore.createFileSystemPathStream(pathPattern)
		: eventStore.fileSystemEvents;
	
	const eventCount = writable(0);
	const lastEvent = writable<any>(null);
	
	let unsubscribe: (() => void) | null = null;
	
	function start() {
		if (!enabled || unsubscribe) return;
		
		// Subscribe to file system events
		unsubscribe = fileSystemEvents.subscribe(events => {
			const latestEvent = events[events.length - 1];
			if (latestEvent) {
				eventCount.update(count => count + 1);
				lastEvent.set(latestEvent);
				
				// Call user callback if provided
				callback?.(latestEvent);
			}
		});
	}
	
	function stop() {
		if (unsubscribe) {
			unsubscribe();
			unsubscribe = null;
		}
	}
	
	// Auto-start if enabled
	if (enabled) {
		start();
	}
	
	return {
		fileSystemEvents,
		eventCount,
		lastEvent,
		start,
		stop,
		destroy: stop
	};
}