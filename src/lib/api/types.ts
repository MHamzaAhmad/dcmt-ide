// API Types and Interfaces

export interface LLMModel {
	id: string;
	name: string;
	description?: string;
}

export interface ModelsResponse {
	models: LLMModel[];
}

export interface ChatMessage {
	id: string;
	role: 'user' | 'assistant' | 'system';
	content: string;
	timestamp: Date;
	model?: string;
	metadata?: { type?: string; [key: string]: any };
}

export interface ChatRequest {
	model: string;
	messages: ChatMessage[];
	temperature?: number;
	max_tokens?: number;
}

export interface ChatResponse {
	id: string;
	model: string;
	message: ChatMessage;
	usage?: {
		prompt_tokens: number;
		completion_tokens: number;
		total_tokens: number;
	};
}

export interface APIError {
	error: string;
	message: string;
	status: number;
}


// Canonical LaTeX build state (snapshot)
export type LatexBuildPhase = 'idle' | 'queued' | 'started' | 'success' | 'error';

export interface LatexBuildState {
	main_file: string | null;
	phase: LatexBuildPhase;
	pdf_path: string | null;
	pdf_version: number;
	engine: string | null;
	errors: string[] | null;
	started_at: number | null;
	finished_at: number | null;
	session_id: string;
}
// File System Types (shared between desktop and web)
export interface FileInfo {
	name: string;
	path: string;
	file_type: 'File' | 'Directory';
	size?: number;
	modified?: number;
	children?: FileInfo[];
}

export interface FileContent {
	path: string;
	content: string;
	size: number;
	modified: number;
}

export interface CreateFileRequest {
	path: string;
	content?: string;
	is_dir: boolean;
}

export interface UpdateFileRequest {
	content: string;
}

// Project Types (desktop only)
export interface ProjectInfo {
	name: string;
	path: string;
	selected_at: number;
}

// Generic Platform API Interface (will support filesystem, compilation, etc.)
export interface PlatformAPI {
	// Project management (desktop only)
	selectProjectFolder?(): Promise<ProjectInfo | null>;
	getCurrentProject?(): Promise<ProjectInfo | null>;
	clearProject?(): Promise<void>;
	
	// File operations
	getDirectoryTree(path: string): Promise<FileInfo>;
	readFileContent(path: string): Promise<FileContent>;
	readFileRaw(path: string): Promise<string>;
	writeFileContent(path: string, content: string): Promise<void>;
	createFile(path: string, content?: string, isDirectory?: boolean): Promise<void>;
	deleteFile(path: string): Promise<void>;
	renameFile(oldPath: string, newPath: string): Promise<void>;
	fileExists(path: string): Promise<boolean>;
	
	// LaTeX compilation
	compileLatex?(request: LaTeXCompileRequest): Promise<LaTeXCompileResponse>;
	findMainLatexFile?(): Promise<string>;
	getLatexStatus?(): Promise<LatexBuildState>;
	
	// Compilation events
	onCompilationEvent?(callback: (event: CompilationEvent) => void): () => void;
	setAutoCompile?(enabled: boolean): Promise<void>;
	setMainFile?(filePath: string | null): Promise<void>;
}

// Specific interfaces for different API categories
export interface FileSystemOperations {
	getDirectoryTree(path: string): Promise<FileInfo>;
	readFileContent(path: string): Promise<FileContent>;
	readFileRaw(path: string): Promise<string>; // Returns URL to raw file data
	writeFileContent(path: string, content: string): Promise<void>;
	createFile(path: string, content?: string, isDirectory?: boolean): Promise<void>;
	deleteFile(path: string): Promise<void>;
	renameFile(oldPath: string, newPath: string): Promise<void>;
	fileExists(path: string): Promise<boolean>;
}

export interface ProjectOperations {
	selectProjectFolder?(): Promise<ProjectInfo | null>;
	getCurrentProject?(): Promise<ProjectInfo | null>;
	clearProject?(): Promise<void>;
}

export interface LaTeXOperations {
	compileLatex(request: LaTeXCompileRequest): Promise<LaTeXCompileResponse>;
	findMainLatexFile(): Promise<string>;
	getLatexStatus(): Promise<LatexBuildState>;
}

export interface GitOperations {
	getStatus(): Promise<GitStatus>;
	getDiff(staged: boolean): Promise<GitDiff>;
	generateSummary(staged: boolean): Promise<CommitSummary>;
	stageFiles(paths: string[]): Promise<void>;
	stageAll(): Promise<void>;
	commit(message: string): Promise<CommitResult>;
	push(): Promise<void>;
	commitAndPush(message: string): Promise<CommitResult>;
}

// File Watcher Interface
export interface FileWatcherOperations {
	isActive(): boolean;
	destroy(): Promise<void>;
}

// File Event Types for unified handling
export interface FileEventData {
	event_type: 'Created' | 'Modified' | 'Deleted' | 'Renamed';
	path: string;
	metadata: {
		is_directory: boolean;
		size?: number;
		old_path?: string;
		new_path?: string;
	};
	timestamp: number;
}

// LaTeX Compilation Types
export interface LaTeXCompileRequest {
	provider: LaTeXProvider;
}

export interface LaTeXCompileResponse {
	success: boolean;
	message: string;
	errors?: string[];
	output_file?: string;
}

export enum LaTeXProvider {
	Auto = 'auto',
	Pdflatex = 'pdflatex',
	Xelatex = 'xelatex',
	Lualatex = 'lualatex'
}

export interface LaTeXCompileError {
	error: string;
	message: string;
	line?: number;
	file?: string;
}

export interface CompilationEvent {
	id: string;
	event_type: 'queued' | 'started' | 'success' | 'error' | 'main_file_detected';
	main_file: string;
	timestamp: number;
	metadata?: {
		reason?: string;
		pdf_path?: string;
		errors?: string[];
		engine?: string;
		duration_ms?: number;
	};
}

// Agent System Types
export interface LiteLLMModel {
	id: string;
	object: string;
	created: number;
	owned_by: string;
}

export interface LiteLLMModelsResponse {
	data: LiteLLMModel[];
	object: string;
}

export interface AgentChatRequest {
	session_id: string;
	message: string;
	model: string;
}

export interface AgentChatResponse {
	session_id: string;
	job_id: string;
}

export interface AgentSessionInfo {
	id: string;
	user_id?: string;
	message_count: number;
	created_at: string;
	last_activity: string;
}

export interface AgentToolDefinition {
	type: string; // "function"
	function: {
		name: string;
		description: string;
		parameters: any; // JSON schema
	};
}

// Base metadata fields that are flattened into all agent events
export interface AgentEventMetadata {
	event_id: string;
	operation_id: string;
	timestamp: number;
	is_file_operation: boolean;
	file_paths: string[];
}

// Agent Event Types for real-time updates (with flattened metadata)
export type AgentEvent = 
	| ({ type: 'JobQueued'; job_id: string; session_id: string } & AgentEventMetadata)
	| ({ type: 'LLMCallStart'; model: string } & AgentEventMetadata)
	| ({ type: 'LLMStreaming'; content: string } & AgentEventMetadata)
	| ({ type: 'StreamChunk'; content: string } & AgentEventMetadata)
	| ({ type: 'ToolCallStart'; tool_id: string; tool_name: string } & AgentEventMetadata)
	| ({ type: 'ToolCallReady'; tool_call: AgentToolCall } & AgentEventMetadata)
	| ({ type: 'ToolCallRequested'; tool: string; args: any; display_name?: string; progressive_form?: string } & AgentEventMetadata)
	| ({ type: 'ToolExecuting'; tool: string; display_name?: string; progressive_form?: string } & AgentEventMetadata)
	| ({ type: 'ToolCompleted'; tool: string; result: string } & AgentEventMetadata)
	| ({ type: 'ParallelToolsStart'; count: number } & AgentEventMetadata)
	| ({ type: 'ParallelToolsComplete'; count: number } & AgentEventMetadata)
	| ({ type: 'LLMCallComplete' } & AgentEventMetadata)
	| ({ type: 'JobComplete'; response: string } & AgentEventMetadata)
	| ({ type: 'Error'; message: string } & AgentEventMetadata);

// Enhanced ChatMessage for agent support
export interface AgentChatMessage extends ChatMessage {
	tool_calls?: AgentToolCall[];
	tool_call_id?: string;
	streaming?: boolean;
	job_id?: string;
	status?: 'sending' | 'streaming' | 'tool_execution' | 'completed' | 'error';
	error?: string;
}

export interface AgentToolCall {
	id: string;
	type: string;
	function: {
		name: string;
		arguments: string; // JSON string
	};
}

export interface AgentToolResult {
	tool_call_id: string;
	tool_name: string;
	status: 'executing' | 'completed' | 'error';
	result?: string;
	error?: string;
	started_at?: Date;
	completed_at?: Date;
}

// Agent Operations Interface
export interface AgentOperations {
	listModels(): Promise<LiteLLMModelsResponse>;
	sendMessage(request: AgentChatRequest): Promise<AgentChatResponse>;
	subscribeToEvents(sessionId: string): Promise<void>;
	unsubscribeFromEvents(sessionId: string): Promise<void>;
	getSessionInfo(sessionId: string): Promise<AgentSessionInfo | null>;
	listSessions(): Promise<AgentSessionInfo[]>;
	clearSession(sessionId: string): Promise<boolean>;
	getAvailableTools(): Promise<AgentToolDefinition[]>;
	isAgentAvailable(): Promise<boolean>;
	getCurrentSessionId(): string | null;
	setCurrentSessionId(sessionId: string): void;
}

// Git Version Control Types
export interface GitStatus {
	branch: string;
	ahead: number;
	behind: number;
	staged: GitFileStatus[];
	unstaged: GitFileStatus[];
	untracked: string[];
}

export interface GitFileStatus {
	path: string;
	status: 'added' | 'modified' | 'deleted' | 'renamed';
	additions: number;
	deletions: number;
}

export interface GitDiff {
	files: FileDiff[];
	stats: DiffStats;
}

export interface FileDiff {
	path: string;
	old_path?: string;
	status: 'added' | 'modified' | 'deleted' | 'renamed';
	additions: number;
	deletions: number;
	hunks: string[];
}

export interface DiffStats {
	additions: number;
	deletions: number;
	files_changed: number;
}

export interface CommitResult {
	sha: string;
	message: string;
	author: string;
	timestamp: number;
}

export interface CommitSummary {
	summary: string;
	bullets: string[];
	suggestedMessage: string;
}

// Billing / Limits
export interface BenefitInfo {
	id: string;
	benefit_id: string;
	benefit_type: string;
	description: string;
	metadata: any;
}

export interface LimitsResponse {
	has_active_subscription: boolean;
	benefits: BenefitInfo[];
}

// Billing Operations (unified across platforms)
export interface BillingOperations {
	getLimits(): Promise<LimitsResponse>;
}