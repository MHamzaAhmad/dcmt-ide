// API Client Configuration

import { authStore } from '$lib/stores/auth';
import { getApiBaseUrl } from '$lib/utils/api';

export class APIClient {
	public baseURL: string;

	constructor(baseURL: string = '') {
		this.baseURL = baseURL || getApiBaseUrl();
	}

	private async request<T>(
		endpoint: string,
		options: RequestInit = {}
	): Promise<T> {
		const url = `${this.baseURL}${endpoint}`;

		const headers: Record<string, string> = {
			'Content-Type': 'application/json',
			...(options.headers as Record<string, string> || {}),
		};

		// Get fresh token from authStore for each request
		const token = await authStore.getToken();
		if (token) {
			headers['Authorization'] = `Bearer ${token}`;
		}

		const config: RequestInit = {
			...options,
			headers,
		};

		try {
			const response = await fetch(url, config);

			if (!response.ok) {
				const error = await response.json().catch(() => ({
					error: 'Request failed',
					message: response.statusText,
					status: response.status
				}));
				throw new Error(error.message || `Request failed with status ${response.status}`);
			}

			return await response.json();
		} catch (error) {
			if (error instanceof Error) {
				throw error;
			}
			throw new Error('An unknown error occurred');
		}
	}

	async get<T>(endpoint: string): Promise<T> {
		return this.request<T>(endpoint, { method: 'GET' });
	}

	async post<T>(endpoint: string, body: any): Promise<T> {
		return this.request<T>(endpoint, {
			method: 'POST',
			body: JSON.stringify(body),
		});
	}

	async put<T>(endpoint: string, body: any): Promise<T> {
		return this.request<T>(endpoint, {
			method: 'PUT',
			body: JSON.stringify(body),
		});
	}

	async delete<T>(endpoint: string): Promise<T> {
		return this.request<T>(endpoint, { method: 'DELETE' });
	}

	setBaseURL(url: string) {
		this.baseURL = url;
	}
}

// Create singleton instance
export const apiClient = new APIClient();