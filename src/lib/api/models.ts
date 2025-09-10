// Models API Endpoints

import { apiClient } from './client';
import { liteLLMClient } from './litellm';
import type { ModelsResponse, ChatRequest, ChatResponse, LiteLLMModelsResponse, LiteLLMModel, LLMModel } from './types';

export const modelsAPI = {
	/**
	 * List all available LLM models from LiteLLM
	 */
	async listModels(): Promise<ModelsResponse> {
		try {
			// Use centralized LiteLLM client
			const liteLLMResponse = await liteLLMClient.fetchModels();
			
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
		// Use centralized LiteLLM client
		return await liteLLMClient.fetchModels();
	},

	/**
	 * Send a chat request to the selected model
	 */
	async sendChatMessage(request: ChatRequest): Promise<ChatResponse> {
		return apiClient.post<ChatResponse>('/llm/chat', request);
	}
};