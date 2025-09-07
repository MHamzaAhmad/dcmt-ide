// Models API Endpoints

import { apiClient } from './client';
import type { ModelsResponse, ChatRequest, ChatResponse, LiteLLMModelsResponse, LiteLLMModel, LLMModel } from './types';

export const modelsAPI = {
	/**
	 * List all available LLM models from LiteLLM
	 */
	async listModels(): Promise<ModelsResponse> {
		try {
			// Try to fetch from LiteLLM directly
			const liteLLMUrl = import.meta.env.VITE_LITELLM_BASE_URL || 'http://localhost:4000';
			const response = await fetch(`${liteLLMUrl}/v1/models`, {
				headers: {
					'Authorization': `Bearer ${import.meta.env.VITE_LITELLM_API_KEY || 'your-api-key'}`
				}
			});
			
			if (!response.ok) {
				throw new Error(`HTTP ${response.status}: ${response.statusText}`);
			}
			
			const liteLLMResponse: LiteLLMModelsResponse = await response.json();
			
			// Transform LiteLLM response to our format
			const models: LLMModel[] = liteLLMResponse.data.map((model: LiteLLMModel) => ({
				id: model.id,
				name: model.id.replace(/^gpt-/, 'GPT-').replace(/^claude-/, 'Claude ').replace(/^gemini-/, 'Gemini '),
				description: `${model.owned_by} model`
			}));
			
			return { models };
		} catch (error) {
			console.error('Failed to fetch models from LiteLLM:', error);
			
			// Fallback to static models if LiteLLM is not available
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
	 * List models with raw LiteLLM format
	 */
	async listLiteLLMModels(): Promise<LiteLLMModelsResponse> {
		const liteLLMUrl = import.meta.env.VITE_LITELLM_BASE_URL || 'http://localhost:4000';
		const response = await fetch(`${liteLLMUrl}/v1/models`, {
			headers: {
				'Authorization': `Bearer ${import.meta.env.VITE_LITELLM_API_KEY || 'your-api-key'}`
			}
		});
		
		if (!response.ok) {
			throw new Error(`HTTP ${response.status}: ${response.statusText}`);
		}
		
		return await response.json();
	},

	/**
	 * Send a chat request to the selected model
	 */
	async sendChatMessage(request: ChatRequest): Promise<ChatResponse> {
		return apiClient.post<ChatResponse>('/llm/chat', request);
	}
};