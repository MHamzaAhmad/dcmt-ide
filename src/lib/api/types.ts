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