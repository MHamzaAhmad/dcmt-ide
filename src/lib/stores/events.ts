// Unified Event System - Central event hub for the entire application
import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import type { QueryClient } from '@tanstack/svelte-query';
import { fileSystemKeys } from '$lib/api/hooks/useFileSystem';

// ============================================================================
// Event Type Definitions
// ============================================================================

export interface FileSystemEvent {
  type: 'filesystem';
  subtype: 'file_created' | 'file_modified' | 'file_deleted' | 'file_renamed';
  payload: {
    path: string;
    oldPath?: string;
    isDirectory: boolean;
    size?: number;
    timestamp: number;
    source: 'agent' | 'user' | 'watcher';
  };
}

export interface AgentEvent {
  type: 'agent';
  subtype: 'job_queued' | 'llm_call_start' | 'llm_streaming' | 'tool_call_requested' 
         | 'tool_executing' | 'tool_completed' | 'parallel_tools_start' 
         | 'parallel_tools_complete' | 'llm_call_complete' | 'job_complete' | 'error';
  payload: {
    sessionId: string;
    tool?: string;
    result?: any;
    message?: string;
    timestamp: number;
  };
}

export interface CompilationEvent {
  type: 'compilation';
  subtype: 'queued' | 'started' | 'completed' | 'failed';
  payload: {
    mainFile: string;
    pdfPath?: string;
    errors?: string[];
    warnings?: string[];
    timestamp: number;
    reason?: string;
  };
}

export interface ConnectionEvent {
  type: 'connection';
  subtype: 'websocket_connected' | 'websocket_disconnected' | 'websocket_error' 
         | 'tauri_connected' | 'api_error';
  payload: {
    connectionId?: string;
    error?: string;
    timestamp: number;
  };
}

export interface UIEvent {
  type: 'ui';
  subtype: 'file_opened' | 'file_closed' | 'project_changed' | 'theme_changed';
  payload: {
    filePath?: string;
    projectPath?: string;
    theme?: string;
    timestamp: number;
  };
}

export type SystemEvent = FileSystemEvent | AgentEvent | CompilationEvent | ConnectionEvent | UIEvent;

// ============================================================================
// Event Store State
// ============================================================================

export interface EventStoreState {
  // Event streams
  allEvents: SystemEvent[];
  
  // Connection status
  connections: {
    websocket: 'connected' | 'disconnected' | 'connecting' | 'error';
    tauri: 'connected' | 'disconnected' | 'error';
  };
  
  // Query invalidation queue
  queryInvalidationQueue: Array<{
    path: string;
    changeType: string;
    timestamp: number;
  }>;
  
  // Event statistics
  eventCounts: Record<string, number>;
  
  // Configuration
  enableDebugLogging: boolean;
  maxEventHistory: number;
}

// ============================================================================
// Event Store Implementation
// ============================================================================

function createEventStore() {
  const initialState: EventStoreState = {
    allEvents: [],
    connections: {
      websocket: 'disconnected',
      tauri: 'disconnected'
    },
    queryInvalidationQueue: [],
    eventCounts: {},
    enableDebugLogging: false,
    maxEventHistory: 1000
  };

  const { subscribe, set, update } = writable<EventStoreState>(initialState);
  
  // Query client instance for invalidation
  let queryClient: QueryClient | null = null;
  
  // Event debouncing for query invalidation
  const debounceTimers = new Map<string, ReturnType<typeof setTimeout>>();
  const DEBOUNCE_MS = 100;

  // ============================================================================
  // Core Event Methods
  // ============================================================================

  function emit(event: SystemEvent) {
    const timestamp = Date.now();
    const eventWithTimestamp: SystemEvent = {
      ...event,
      payload: {
        ...event.payload,
        timestamp
      }
    } as SystemEvent;

    update(state => {
      const newEvents = [...state.allEvents, eventWithTimestamp].slice(-state.maxEventHistory);
      const eventKey = `${event.type}:${event.subtype}`;
      const newEventCounts = {
        ...state.eventCounts,
        [eventKey]: (state.eventCounts[eventKey] || 0) + 1
      };

      if (state.enableDebugLogging) {
        console.log(`[EventStore] ${eventKey}:`, event.payload);
      }

      return {
        ...state,
        allEvents: newEvents,
        eventCounts: newEventCounts
      };
    });

    // Handle cross-cutting concerns
    handleQueryInvalidation(eventWithTimestamp);
    handleNotifications(eventWithTimestamp);
  }

  // ============================================================================
  // Event Filtering and Subscription
  // ============================================================================

  function createEventStream<T extends SystemEvent>(
    predicate: (event: SystemEvent) => event is T
  ) {
    return derived({ subscribe }, ($store) => 
      $store.allEvents.filter(predicate)
    );
  }

  // Specialized event streams
  const fileSystemEvents = createEventStream((event): event is FileSystemEvent => 
    event.type === 'filesystem'
  );

  const agentEvents = createEventStream((event): event is AgentEvent => 
    event.type === 'agent'
  );

  const compilationEvents = createEventStream((event): event is CompilationEvent => 
    event.type === 'compilation'
  );

  const connectionEvents = createEventStream((event): event is ConnectionEvent => 
    event.type === 'connection'
  );

  const uiEvents = createEventStream((event): event is UIEvent => 
    event.type === 'ui'
  );

  // Session-specific agent events
  function createAgentSessionStream(sessionId: string) {
    return derived(agentEvents, ($agentEvents) =>
      $agentEvents.filter(event => event.payload.sessionId === sessionId)
    );
  }

  // Path-specific filesystem events
  function createFileSystemPathStream(pathPattern: string | RegExp) {
    return derived(fileSystemEvents, ($fileSystemEvents) =>
      $fileSystemEvents.filter(event => {
        if (typeof pathPattern === 'string') {
          return event.payload.path.includes(pathPattern);
        }
        return pathPattern.test(event.payload.path);
      })
    );
  }

  // Generic event stream aliases for backwards compatibility
  function createFileSystemEventStream() {
    return fileSystemEvents;
  }

  function createCompilationEventStream() {
    return compilationEvents;
  }

  // ============================================================================
  // Query Invalidation Logic
  // ============================================================================

  function handleQueryInvalidation(event: SystemEvent) {
    if (!queryClient) return;

    if (event.type === 'filesystem') {
      const { path, oldPath, isDirectory } = event.payload;
      
      // Clear any existing debounce timer for this path
      const existingTimer = debounceTimers.get(path);
      if (existingTimer) {
        clearTimeout(existingTimer);
      }

      // Set up debounced invalidation
      const timer = setTimeout(() => {
        invalidateFileSystemQueries(path, event.subtype, { oldPath, isDirectory });
        debounceTimers.delete(path);
      }, DEBOUNCE_MS);

      debounceTimers.set(path, timer);
      
      // Update invalidation queue for debugging
      update(state => ({
        ...state,
        queryInvalidationQueue: [
          ...state.queryInvalidationQueue.slice(-49), // Keep last 50
          {
            path,
            changeType: event.subtype,
            timestamp: event.payload.timestamp
          }
        ]
      }));
    }

    if (event.type === 'compilation' && event.subtype === 'completed') {
      // Invalidate PDF-related queries
      if (event.payload.pdfPath) {
        queryClient.invalidateQueries({
          predicate: (query) => {
            const key = query.queryKey as string[];
            return key.some(part => typeof part === 'string' && part.includes('.pdf'));
          }
        });
      }
    }
  }

  function invalidateFileSystemQueries(
    path: string, 
    changeType: string, 
    meta: { oldPath?: string; isDirectory?: boolean }
  ) {
    if (!queryClient) return;

    console.log(`[EventStore] Invalidating queries for ${changeType} on path: ${path}`);
    
    const pathParts = path.split('/').filter(part => part);
    
    // Invalidate directory tree queries for all parent directories
    for (let i = 0; i <= pathParts.length; i++) {
      const directoryPath = pathParts.slice(0, i).join('/');
      queryClient.invalidateQueries({ 
        queryKey: fileSystemKeys.directoryTree(directoryPath) 
      });
    }

    // Handle file content invalidation
    if (!meta.isDirectory) {
      queryClient.invalidateQueries({ 
        queryKey: fileSystemKeys.fileContent(path) 
      });
      
      // Remove from cache for fresh fetch
      queryClient.removeQueries({
        queryKey: fileSystemKeys.fileContent(path)
      });
    }

    // Handle specific change types
    switch (changeType) {
      case 'file_created':
      case 'file_deleted':
        // Invalidate entire file system structure
        queryClient.invalidateQueries({
          predicate: (query) => {
            const key = query.queryKey as string[];
            return key[0] === 'filesystem' || key[0] === 'directory-tree';
          }
        });
        break;
        
      case 'file_renamed':
        // Remove old file from cache
        if (meta.oldPath) {
          queryClient.removeQueries({ 
            queryKey: fileSystemKeys.fileContent(meta.oldPath) 
          });
          
          const oldParentPath = meta.oldPath.split('/').slice(0, -1).join('/');
          queryClient.invalidateQueries({ 
            queryKey: fileSystemKeys.directoryTree(oldParentPath) 
          });
        }
        break;
    }
  }

  // ============================================================================
  // Notification Handling
  // ============================================================================

  function handleNotifications(event: SystemEvent) {
    // Future: Handle system notifications, toast messages, etc.
    // For now, just log important events
    
    if (event.type === 'agent' && event.subtype === 'error') {
      console.error('[EventStore] Agent error:', event.payload.message);
    }
    
    if (event.type === 'compilation' && event.subtype === 'failed') {
      console.error('[EventStore] Compilation failed:', event.payload.errors);
    }
    
    if (event.type === 'connection' && event.subtype === 'websocket_disconnected') {
      console.warn('[EventStore] WebSocket disconnected');
    }
  }

  // ============================================================================
  // Utility Methods
  // ============================================================================

  function setQueryClient(client: QueryClient) {
    queryClient = client;
  }

  function setDebugLogging(enabled: boolean) {
    update(state => ({
      ...state,
      enableDebugLogging: enabled
    }));
  }

  function updateConnectionStatus(
    type: 'websocket' | 'tauri',
    status: 'connected' | 'disconnected' | 'connecting' | 'error'
  ) {
    update(state => ({
      ...state,
      connections: {
        ...state.connections,
        [type]: status
      }
    }));

    // Emit connection event
    emit({
      type: 'connection',
      subtype: `${type}_${status}` as ConnectionEvent['subtype'],
      payload: {
        timestamp: Date.now()
      }
    });
  }

  function clearEventHistory() {
    update(state => ({
      ...state,
      allEvents: [],
      eventCounts: {},
      queryInvalidationQueue: []
    }));
  }

  function getEventStats() {
    const state = get({ subscribe });
    return {
      totalEvents: state.allEvents.length,
      eventCounts: state.eventCounts,
      connections: state.connections,
      queuedInvalidations: state.queryInvalidationQueue.length
    };
  }

  // ============================================================================
  // Convenience Event Emitters
  // ============================================================================

  const events = {
    // File system events
    fileCreated: (path: string, isDirectory = false, source: 'agent' | 'user' | 'watcher' = 'user') =>
      emit({
        type: 'filesystem',
        subtype: 'file_created',
        payload: { path, isDirectory, source, timestamp: Date.now() }
      }),

    fileModified: (path: string, size?: number, source: 'agent' | 'user' | 'watcher' = 'user') =>
      emit({
        type: 'filesystem',
        subtype: 'file_modified',
        payload: { path, size, isDirectory: false, source, timestamp: Date.now() }
      }),

    fileDeleted: (path: string, isDirectory = false, source: 'agent' | 'user' | 'watcher' = 'user') =>
      emit({
        type: 'filesystem',
        subtype: 'file_deleted',
        payload: { path, isDirectory, source, timestamp: Date.now() }
      }),

    fileRenamed: (path: string, oldPath: string, isDirectory = false, source: 'agent' | 'user' | 'watcher' = 'user') =>
      emit({
        type: 'filesystem',
        subtype: 'file_renamed',
        payload: { path, oldPath, isDirectory, source, timestamp: Date.now() }
      }),

    // Agent events
    agentToolExecuting: (sessionId: string, tool: string) =>
      emit({
        type: 'agent',
        subtype: 'tool_executing',
        payload: { sessionId, tool, timestamp: Date.now() }
      }),

    agentToolCompleted: (sessionId: string, tool: string, result: any) =>
      emit({
        type: 'agent',
        subtype: 'tool_completed',
        payload: { sessionId, tool, result, timestamp: Date.now() }
      }),

    agentError: (sessionId: string, message: string) =>
      emit({
        type: 'agent',
        subtype: 'error',
        payload: { sessionId, message, timestamp: Date.now() }
      }),

    // Compilation events
    compilationQueued: (mainFile: string, reason?: string) =>
      emit({
        type: 'compilation',
        subtype: 'queued',
        payload: { mainFile, reason, timestamp: Date.now() }
      }),

    compilationCompleted: (mainFile: string, pdfPath: string) =>
      emit({
        type: 'compilation',
        subtype: 'completed',
        payload: { mainFile, pdfPath, timestamp: Date.now() }
      }),

    compilationFailed: (mainFile: string, errors: string[]) =>
      emit({
        type: 'compilation',
        subtype: 'failed',
        payload: { mainFile, errors, timestamp: Date.now() }
      }),

    // UI events
    fileOpened: (filePath: string) =>
      emit({
        type: 'ui',
        subtype: 'file_opened',
        payload: { filePath, timestamp: Date.now() }
      }),

    projectChanged: (projectPath: string) =>
      emit({
        type: 'ui',
        subtype: 'project_changed',
        payload: { projectPath, timestamp: Date.now() }
      })
  };

  // ============================================================================
  // Return Store Interface
  // ============================================================================

  return {
    // Core store
    subscribe,
    
    // Event emission
    emit,
    events,
    
    // Event streams
    fileSystemEvents,
    agentEvents,
    compilationEvents,
    connectionEvents,
    uiEvents,
    createAgentSessionStream,
    createFileSystemPathStream,
    createFileSystemEventStream,
    createCompilationEventStream,
    
    // Configuration
    setQueryClient,
    setDebugLogging,
    updateConnectionStatus,
    
    // Utilities
    clearEventHistory,
    getEventStats
  };
}

// ============================================================================
// Exports
// ============================================================================

export const eventStore = createEventStore();