import { writable } from 'svelte/store';
import { apiClient } from '$lib/api/client';
import { getApiBaseUrl } from '$lib/utils/api';

export interface APIState {
	baseURL: string;
	isConnected: boolean;
	connectionError: string | null;
}

function createAPIStore() {
	const initialState: APIState = {
		baseURL: getApiBaseUrl(),
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
	};
}

export const apiStore = createAPIStore();