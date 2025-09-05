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