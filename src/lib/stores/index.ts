/**
 * Unified Store System - Main exports
 * Reactive, scalable architecture for DCMT Editor
 */

// Core stores
export { workspaceStore } from './workspace';
export { latexStore } from './latex';
export { pdfStore } from './pdf';
export { agentStore } from './agent';

// Orchestrator
export { orchestrator, initializationOrchestrator } from './orchestrator';

// Legacy stores (for gradual migration)
export { fileTree, openFiles } from './files';
// export { editorStore } from './editor';
export { chatStore } from './chat';
// export { themeStore } from './theme';
// export { apiStore } from './api';

// Types
export type { FileContent, FileTreeNode, WorkspaceState } from './workspace';
export type { LaTeXState, LaTeXCompilationResult } from './latex';
export type { PDFState, PDFDocument, PDFViewerState } from './pdf';
export type { AgentState } from './agent';
export type { InitializationState, InitializationStep, InitializationOptions } from './orchestrator';