// Models API Endpoints

import { apiClient } from './client';
import type { ModelsResponse, ChatRequest, ChatResponse } from './types';

export const modelsAPI = {
	/**
	 * List all available LLM models from backend API
	 */
	async listModels(): Promise<ModelsResponse> {
		try {
			// Use backend API endpoint instead of direct LiteLLM call
			const response = await apiClient.get<ModelsResponse>('/api/llm/models');
			return response;
		} catch (error) {
			console.error('Failed to fetch models from backend API:', error);

			// Fallback to static models if backend API is not available
			return {
				models: [
					{ id: 'gpt-4-turbo', name: 'GPT-4 Turbo', description: 'Most capable GPT-4 model' },
					{ id: 'gpt-4', name: 'GPT-4', description: 'Standard GPT-4 model' },
					{ id: 'claude-3-sonnet', name: 'Claude 3 Sonnet', description: 'Anthropic Claude model' },
					{ id: 'gemini-pro', name: 'Gemini Pro', description: 'Google Gemini model' }
				]
			};
		}
	},

	/**
	 * Send a chat request to the selected model via backend API
	 */
	async sendChatMessage(request: ChatRequest): Promise<ChatResponse> {
		return apiClient.post<ChatResponse>('/api/llm/chat', request);
	}
};