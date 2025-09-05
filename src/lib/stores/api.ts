import { writable } from 'svelte/store';
import { apiClient } from '$lib/api/client';

export interface APIState {
	baseURL: string;
	isConnected: boolean;
	connectionError: string | null;
}

function createAPIStore() {
	const initialState: APIState = {
		baseURL: import.meta.env.VITE_API_BASE_URL || 'http://localhost:3001',
		isConnected: false,
		connectionError: null
	};

	const { subscribe, set, update } = writable<APIState>(initialState);

	return {
		subscribe,
		set,
		update,
		
		setBaseURL: (url: string) => {
			apiClient.setBaseURL(url);
			update(state => ({
				...state,
				baseURL: url
			}));
		},
		
		setConnectionStatus: (connected: boolean, error: string | null = null) => {
			update(state => ({
				...state,
				isConnected: connected,
				connectionError: error
			}));
		},
		
		// Test connection to API
		testConnection: async () => {
			try {
				// Try to fetch models as a connection test
				const response = await fetch(`${initialState.baseURL}/api/llm/models`);
				const connected = response.ok;
				
				update(state => ({
					...state,
					isConnected: connected,
					connectionError: connected ? null : 'Failed to connect to API'
				}));
				
				return connected;
			} catch (error) {
				update(state => ({
					...state,
					isConnected: false,
					connectionError: error instanceof Error ? error.message : 'Connection failed'
				}));
				return false;
			}
		}
	};
}

export const apiStore = createAPIStore();