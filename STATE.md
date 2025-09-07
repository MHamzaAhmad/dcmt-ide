# DCMT Editor - Reactive Store Architecture

## Overview

DCMT Editor uses a **reactive store architecture** that provides predictable, scalable state management. This document outlines the current system design, data flow, and integration patterns.

## 🏗️ Architecture Principles

### 1. **Single Source of Truth**
Each domain (files, LaTeX, PDF, agent) has one centralized store that manages all related state.

### 2. **Reactive Data Flow**
Changes flow automatically through the system:
```
File Change → WorkspaceStore → LaTeXStore → PDFStore → UI Updates
```

### 3. **Predictable Initialization**
All stores initialize in a specific order with dependency management via the InitializationOrchestrator.

### 4. **Clean Integration**
Agent operations integrate seamlessly by updating stores, which trigger the entire reactive chain.

## 🗂️ Store Structure

### Core Stores

#### **WorkspaceStore** (`src/lib/stores/workspace.ts`)
- **Purpose**: Centralized file and workspace management
- **Responsibilities**:
  - File content storage and caching
  - Open files and active file tracking
  - LaTeX file detection and main file identification
  - Agent file operation integration
- **Key State**:
  ```typescript
  interface WorkspaceState {
    isReady: boolean;
    files: Map<string, FileContent>;
    openFiles: string[];
    activeFile: string | null;
    mainLatexFile: string | null;
    latexFiles: string[];
  }
  ```
- **Key Methods**:
  - `loadFile(path)` - Load and cache file content
  - `saveFile(path)` - Save file with dirty state tracking
  - `handleAgentFileOperation(tool, path, content)` - Process agent changes

#### **LaTeXStore** (`src/lib/stores/latex.ts`)
- **Purpose**: LaTeX compilation lifecycle management
- **Responsibilities**:
  - Auto-compilation when .tex files change
  - Compilation queuing and debouncing
  - Result tracking with history
  - Integration with workspace file changes
- **Key State**:
  ```typescript
  interface LaTeXState {
    mainFile: string | null;
    compilationStatus: 'idle' | 'queued' | 'compiling' | 'success' | 'error';
    currentPdfPath: string | null;
    lastCompilation: LaTeXCompilationResult | null;
    autoCompile: boolean;
  }
  ```
- **Key Methods**:
  - `scheduleCompilation(reason)` - Queue compilation with debouncing
  - `compileCurrentFile()` - Execute compilation
  - `handleAgentFileOperation(tool, path)` - React to agent changes

#### **PDFStore** (`src/lib/stores/pdf.ts`)
- **Purpose**: PDF preview and viewer management
- **Responsibilities**:
  - PDF.js integration and loading
  - Viewer controls (zoom, pages, rotation)
  - Auto-refresh when LaTeX compilation succeeds
  - Canvas rendering management
- **Key State**:
  ```typescript
  interface PDFState {
    currentPdf: PDFDocument | null;
    isLoading: boolean;
    viewer: PDFViewerState;
    canvas: HTMLCanvasElement | null;
    autoRefresh: boolean;
  }
  ```
- **Key Methods**:
  - `loadPdf(path)` - Load PDF document
  - `setCanvas(canvas)` - Connect canvas for rendering
  - `renderCurrentPage()` - Render current page to canvas
  - Viewer controls: `nextPage()`, `prevPage()`, `zoomIn()`, `zoomOut()`, `rotate()`

#### **AgentStore** (`src/lib/stores/agent.ts`)  
- **Purpose**: Agent system integration
- **Responsibilities**:
  - Agent session management
  - Message handling and streaming
  - Clean integration with other stores
  - File operation coordination
- **Key Integration Methods**:
  - `handleAgentToolOperation(tool, path, result)` - Coordinate with other stores
  - `handleEnhancedFileOperation(operation)` - Process file operations

#### **EventStore** (`src/lib/stores/events.ts`)
- **Purpose**: Unified event system for the entire application
- **Responsibilities**:
  - Central event hub for all system events
  - Reactive event streams and subscriptions
  - Query invalidation coordination
  - Cross-cutting event handling
- **Key State**:
  ```typescript
  interface EventStoreState {
    allEvents: SystemEvent[];
    connections: { websocket: Status; tauri: Status };
    queryInvalidationQueue: QueuedInvalidation[];
    eventCounts: Record<string, number>;
  }
  ```
- **Event Types**:
  - `FileSystemEvent` - File operations (create/modify/delete/rename)
  - `AgentEvent` - Agent tool execution and completion
  - `CompilationEvent` - LaTeX compilation lifecycle
  - `ConnectionEvent` - Network and system connections
  - `UIEvent` - User interface state changes
- **Key Methods**:
  - `emit(event)` - Emit system events
  - `events.*` - Convenience event emitters
  - `createEventStream(predicate)` - Create filtered event streams
  - `setQueryClient(client)` - Configure query invalidation

#### **InitializationOrchestrator** (`src/lib/stores/orchestrator.ts`)
- **Purpose**: Predictable system startup
- **Responsibilities**:
  - Manage initialization order and dependencies
  - Error handling and retry logic
  - Progress tracking
  - System cleanup on shutdown

## 🔄 Data Flow Patterns

### 1. **Unified Event-Driven Architecture**
```mermaid
graph TD
    A[Event Source] --> B[EventStore]
    B --> C[Event Streams]
    C --> D[Store Subscribers]
    C --> E[Component Subscribers]
    C --> F[Query Invalidation]
    D --> G[Store State Updates]
    E --> H[UI Updates]
    F --> I[Fresh Data Fetch]
    
    subgraph "Event Sources"
        J[File Watchers]
        K[WebSocket Adapters]
        L[Agent Operations]
        M[User Actions]
        N[Compilation System]
    end
    
    J --> A
    K --> A  
    L --> A
    M --> A
    N --> A
```

### 2. **Agent File Operations (Updated)**
```mermaid
graph TD
    A[Agent Tool Execution] --> B[WebSocket Adapter]
    B --> C[EventStore.emit FileSystemEvent]
    C --> D[All Stores Subscribe to EventStore]
    D --> E[WorkspaceStore Updates Files]
    D --> F[LaTeXStore Schedules Compilation]
    D --> G[Query Invalidation]
    E --> H[UI Updates Automatically]
    F --> I[PDF Generation]
    G --> J[Fresh Data Fetch]
```

### 3. **Manual File Changes (Updated)**
```mermaid
graph TD
    A[User Edits File] --> B[WorkspaceStore.updateFileContent]
    B --> C[WorkspaceStore.saveFile]
    C --> D[EventStore.events.fileModified]
    D --> E[FileSystem Event Emitted]
    E --> F[LaTeX Store Reacts via EventStore]
    E --> G[Query Invalidation via EventStore]
    F --> H[Auto-compilation if .tex file]
    G --> I[UI Data Refresh]
```

### 4. **System Initialization (Updated)**
```mermaid
graph TD
    A[App Starts] --> B[InitializationOrchestrator.initialize]
    B --> C[EventStore.initialize]
    C --> D[WorkspaceStore.initialize]
    D --> E[LaTeXStore.initialize]
    E --> F[PDFStore.initialize]
    F --> G[AgentStore.initialize]
    G --> H[All Systems Subscribe to EventStore]
    H --> I[System Ready]
    I --> J[UI Renders]
```

## 🛠️ Integration Patterns

### Using Stores in Components

#### **Reactive State Access**
```svelte
<script lang="ts">
  import { workspaceStore, latexStore, pdfStore, eventStore } from '$lib/stores';
  
  // Reactive state
  const workspaceState = $derived($workspaceStore);
  const latexState = $derived($latexStore);  
  const pdfState = $derived($pdfStore);
  
  // Event streams (modern approach)
  const fileSystemEvents = $derived($eventStore.fileSystemEvents);
  const agentEvents = $derived($eventStore.agentEvents);
  const compilationEvents = $derived($eventStore.compilationEvents);
  
  // Derived store access
  const hasValidPdf = $derived($pdfStore.hasValidPdf);
  const canCompile = $derived(latexStore.canCompile());
</script>
```

#### **Event-Driven Components**
```svelte
<script lang="ts">
  import { eventStore } from '$lib/stores';
  
  // Subscribe to specific file events
  const texFileEvents = eventStore.createFileSystemPathStream(/\.tex$/);
  
  // React to events
  $effect(() => {
    const events = $texFileEvents;
    const latestEvent = events[events.length - 1];
    
    if (latestEvent?.subtype === 'file_modified') {
      // Handle LaTeX file changes
      console.log('LaTeX file changed:', latestEvent.payload.path);
    }
  });
  
  // Subscribe to agent session events
  const sessionEvents = eventStore.createAgentSessionStream('session-123');
  
  $effect(() => {
    const events = $sessionEvents;
    events.forEach(event => {
      if (event.subtype === 'tool_completed') {
        // Handle tool completion
        console.log('Agent tool completed:', event.payload.tool);
      }
    });
  });
</script>
```

#### **Store Method Calls**
```svelte
<script lang="ts">
  import { workspaceStore, latexStore, pdfStore } from '$lib/stores';
  
  async function handleFileOpen(path: string) {
    await workspaceStore.openFile(path);
  }
  
  function handleCompile() {
    latexStore.forceCompile();
  }
  
  function handleZoomIn() {
    pdfStore.zoomIn();
  }
</script>
```

#### **System Initialization**
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { orchestrator } from '$lib/stores';
  import { useQueryClient } from '@tanstack/svelte-query';
  
  onMount(async () => {
    const queryClient = useQueryClient();
    
    await orchestrator.initialize({
      rootPath: '',
      queryClient,
      skipAgentInit: false
    });
  });
</script>
```

### Store Extension Patterns

#### **Adding New File Types**
1. Extend `WorkspaceStore` to detect new file types
2. Create new store for the file type (e.g., `MarkdownStore`)
3. Add integration methods to `AgentStore`
4. Update `InitializationOrchestrator` if needed

#### **Adding New Compilation Targets**
1. Extend `LaTeXStore` or create parallel compilation store
2. Add platform API methods for new compiler
3. Integrate with existing reactive flow

## 🧪 Testing Patterns

### Store Testing
```typescript
// Test stores in isolation
import { get } from 'svelte/store';
import { workspaceStore } from '$lib/stores/workspace';

// Test state changes
await workspaceStore.loadFile('test.tex');
const state = get(workspaceStore);
expect(state.files.has('test.tex')).toBe(true);
```

### Integration Testing  
```typescript
// Test reactive chains
await workspaceStore.handleAgentFileOperation('update_file', 'main.tex', 'content');
// Verify LaTeX store reacts
const latexState = get(latexStore);
expect(latexState.compilationStatus).toBe('queued');
```

## 🚨 Common Issues & Solutions

### **Issue**: Stores not initializing properly
**Solution**: Use `InitializationOrchestrator` and check initialization order

### **Issue**: Agent changes not triggering UI updates  
**Solution**: Verify `AgentStore.handleAgentToolOperation()` is called and store integration is working

### **Issue**: PDF not refreshing after compilation
**Solution**: Check LaTeX→PDF store integration and auto-refresh settings

### **Issue**: Race conditions on page refresh
**Solution**: Use `orchestrator.initialize()` and wait for `isReady` state

## 📁 File Structure

```
src/lib/stores/
├── index.ts              # Unified exports
├── events.ts             # 🆕 Unified event system
├── workspace.ts          # File & workspace management  
├── latex.ts              # LaTeX compilation
├── pdf.ts                # PDF preview & viewer
├── agent.ts              # Agent system integration
├── orchestrator.ts       # Initialization management
├── editor.ts             # 🔄 Modernized editor state
├── chat.ts               # 🔄 Modernized chat state  
└── legacy/               # Old stores (deprecated)
    └── files.ts
```

## 🔄 Migration Status

### ✅ **Completed**
- **EventStore** - Unified event system with reactive streams
- **WorkspaceStore** - Centralized file management with EventStore integration
- **LaTeXStore** - Reactive compilation with event emission
- **PDFStore** - Reactive preview with event subscription
- **AgentStore** - Clean integration via EventStore
- **InitializationOrchestrator** - Predictable startup with EventStore initialization
- **WebSocket Adapters** - All emit to EventStore instead of manual dispatching
- **File Watchers** - Desktop and web watchers emit to EventStore
- **Editor/Chat Stores** - Modernized with EventStore subscriptions
- **Query Invalidation** - Centralized in EventStore
- **Manual Event Dispatching** - Removed from components

### 🚧 **In Progress**  
- Component migration to use EventStore directly
- Legacy hook replacement with modern event-driven patterns

### 📋 **Planned**
- EventStore persistence for faster startup
- Advanced event filtering and replay capabilities  
- Performance monitoring for event throughput

## 🎯 Best Practices

### **Do**
- Use the InitializationOrchestrator for system startup
- Access derived state with `$derived($store.derivedProperty)`  
- Let stores handle their own state - don't bypass them
- Use store methods rather than direct state manipulation
- Test reactive chains end-to-end
- **Use EventStore for all cross-system communication**
- **Subscribe to event streams for reactive behavior**
- **Emit events through EventStore, not manual dispatching**

### **Don't**
- Initialize stores manually - use the orchestrator
- Bypass stores for direct API calls in components
- Mix old event system with new reactive stores
- Assume stores are ready without checking initialization
- Create tight coupling between stores
- **Use window.dispatchEvent() for system events**
- **Create custom event buses when EventStore exists**
- **Handle query invalidation manually in components**

## 🔮 Future Considerations

### **Performance**
- Store state persistence for faster startup
- Incremental compilation for large documents
- Virtual scrolling for large file trees

### **Features**  
- Multi-document support
- Real-time collaboration integration
- Plugin system for custom file types

### **Architecture**
- Store composition patterns for complex workflows
- Event sourcing for undo/redo functionality
- WebWorker integration for heavy computations

---

## 🎉 Unified Event-Driven Architecture Summary

The DCMT Editor now uses a **unified event-driven reactive store architecture** that eliminates the previous "patchy" event system:

### **Before (Fragmented)**
- Multiple event systems: WebSocket adapters, CustomEvents, Tauri events
- Manual `window.dispatchEvent()` calls throughout codebase  
- Duplicated query invalidation logic in multiple hooks
- Platform-specific event handling inconsistencies
- Legacy stores disconnected from main reactive system

### **After (Unified)**
- **Single EventStore** as central hub for all system events
- **Reactive event streams** that stores and components subscribe to
- **Automatic coordination** - when an event occurs, all relevant parts react
- **Centralized query invalidation** handled by EventStore
- **Type-safe events** with discriminated union types
- **Platform-agnostic** API for desktop and web

### **Key Benefits Achieved**
1. **Eliminated Patches** - No more manual event dispatching scattered around
2. **Automatic Coordination** - File changes → LaTeX compilation → PDF refresh works seamlessly  
3. **Single Source of Truth** - All events flow through EventStore
4. **Type Safety** - Strongly typed event system with proper contracts
5. **Platform Consistency** - Same event API for web and desktop
6. **Simplified Testing** - Centralized event system easier to test and debug
7. **Performance** - Debounced query invalidation prevents excessive API calls

The system now works exactly like you wanted - when any event occurs (file change, agent operation, compilation, etc.), all related parties automatically react through the EventStore without any manual coordination required.

---

*This document reflects the unified event-driven reactive store architecture. Update when making significant changes to the system design.*