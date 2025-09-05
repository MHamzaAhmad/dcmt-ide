// LLM Models Hooks using Svelte Query
import { createQuery, createMutation } from '@tanstack/svelte-query';
import { modelsAPI } from '../models';
import type { ModelsResponse, ChatRequest, ChatResponse } from '../types';

// Query Keys
export const modelsKeys = {
	all: ['models'] as const,
	list: () => [...modelsKeys.all, 'list'] as const,
};

/**
 * Hook to get available LLM models
 */
export function useModels(enabled: boolean = true) {
	return createQuery({
		queryKey: modelsKeys.list(),
		queryFn: (): Promise<ModelsResponse> => modelsAPI.listModels(),
		enabled,
		staleTime: 5 * 60 * 1000, // 5 minutes
		retry: 2,
	});
}

/**
 * Hook to send chat messages
 */
export function useSendChat() {
	return createMutation({
		mutationFn: (request: ChatRequest): Promise<ChatResponse> => 
			modelsAPI.sendChatMessage(request),
		onError: (error) => {
			console.error('Failed to send chat message:', error);
		},
	});
}