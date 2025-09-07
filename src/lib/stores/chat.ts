import { writable, derived } from 'svelte/store';
import type { ChatMessage, LLMModel } from '$lib/api/types';
import { eventStore } from './events';

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

	// Subscribe to agent events for chat updates
	eventStore.agentEvents.subscribe(events => {
		events.forEach(event => {
			if (event.subtype === 'llm_streaming' && event.payload.message) {
				// Handle streaming responses - this would need proper integration
				console.log('Streaming message:', event.payload.message);
			}
			
			if (event.subtype === 'error' && event.payload.message) {
				// Add error message to chat
				const errorMessage: ChatMessage = {
					id: crypto.randomUUID(),
					role: 'system',
					content: `Error: ${event.payload.message}`,
					timestamp: Date.now(),
					metadata: { type: 'error' }
				};
				
				update(state => ({
					...state,
					messages: [...state.messages, errorMessage]
				}));
			}
		});
	});

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