// API Client Configuration

export class APIClient {
	public baseURL: string;
	private token: string;

	constructor(baseURL: string = '') {
		this.baseURL = baseURL || import.meta.env.VITE_API_BASE_URL || 'http://localhost:3001';
		this.token = ''; // TODO: Add authentication token when auth is implemented
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

		// TODO: Add authentication header when token is available
		if (this.token) {
			headers['Authorization'] = `Bearer ${this.token}`;
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

	setToken(token: string) {
		this.token = token;
	}

	setBaseURL(url: string) {
		this.baseURL = url;
	}
}

// Create singleton instance
export const apiClient = new APIClient();