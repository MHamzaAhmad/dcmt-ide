/**
 * Unified Store System - Main exports
 * Reactive, scalable architecture for DCMT Editor
 */

// Event System
export { eventStore } from './events';

// Core stores
export { authStore, isAuthenticated, currentUser, isAuthLoading } from './auth';
export { projectStore } from './project';
export { workspaceStore } from './workspace';
export { latexStore, isCompiling } from './latex';
export { pdfStore } from './pdf';
export { agentStore } from './agent';
export { gitStore } from './git';

// Orchestrator
export { orchestrator, initializationOrchestrator } from './orchestrator';

// Legacy stores (for gradual migration)
// export { editorStore } from './editor';
export { chatStore } from './chat';
// export { themeStore } from './theme';
// export { apiStore } from './api';

// Types
export type {
    SystemEvent, FileSystemEvent, AgentEvent, CompilationEvent,
    ConnectionEvent, UIEvent, AuthEvent
} from './events';
export type { AuthState } from './auth';
export type { ProjectState } from './project';
export type { FileContent, FileTreeNode, WorkspaceState } from './workspace';
export type { LaTeXState, LaTeXCompilationResult } from './latex';
export type { PDFState, PDFDocument } from './pdf';
export type { AgentState } from './agent';
export type { GitStoreState } from './git';
export type { InitializationState, InitializationStep, InitializationOptions } from './orchestrator';