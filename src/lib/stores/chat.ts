import { writable } from 'svelte/store';
import type { ChatMessage, LLMModel } from '$lib/api/types';

export interface ChatState {
	messages: ChatMessage[];
	selectedModel: LLMModel | null;
	availableModels: LLMModel[];
	isLoading: boolean;
}

function createChatStore() {
	const initialState: ChatState = {
		messages: [],
		selectedModel: null,
		availableModels: [],
		isLoading: false
	};

	const { subscribe, set, update } = writable<ChatState>(initialState);

	return {
		subscribe,
		set,
		update,
		
		addMessage: (message: ChatMessage) => {
			update(state => ({
				...state,
				messages: [...state.messages, message]
			}));
		},
		
		clearMessages: () => {
			update(state => ({
				...state,
				messages: []
			}));
		},
		
		setSelectedModel: (model: LLMModel | null) => {
			update(state => ({
				...state,
				selectedModel: model
			}));
		},
		
		setAvailableModels: (models: LLMModel[]) => {
			update(state => ({
				...state,
				availableModels: models,
				// Auto-select first model if none selected
				selectedModel: state.selectedModel || models[0] || null
			}));
		},
		
		setLoading: (loading: boolean) => {
			update(state => ({
				...state,
				isLoading: loading
			}));
		}
	};
}

export const chatStore = createChatStore();