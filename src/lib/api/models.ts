// Models API Endpoints

import { apiClient } from './client';
import type { ModelsResponse, ChatRequest, ChatResponse } from './types';

export const modelsAPI = {
	/**
	 * List all available LLM models
	 */
	async listModels(): Promise<ModelsResponse> {
		return apiClient.get<ModelsResponse>('/llm/models');
	},

	/**
	 * Send a chat request to the selected model
	 */
	async sendChatMessage(request: ChatRequest): Promise<ChatResponse> {
		return apiClient.post<ChatResponse>('/llm/chat', request);
	}
};