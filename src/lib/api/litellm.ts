// Centralized LiteLLM API Client
// Following STATE.md principles: Single Source of Truth for LiteLLM operations

import type { LiteLLMModelsResponse } from './types';

/**
 * Centralized client for all LiteLLM API interactions
 * Provides a single source of truth for LiteLLM operations across web and desktop platforms
 */
class LiteLLMClient {
    private baseURL: string;
    private token: string = ''; // Empty for now, will implement auth tokens later

    constructor() {
        this.baseURL = import.meta.env.VITE_LITELLM_BASE_URL || 'http://localhost:4000';
    }

    /**
     * Fetch available models from LiteLLM
     */
    async fetchModels(): Promise<LiteLLMModelsResponse> {
        try {
            const response = await fetch(`${this.baseURL}/v1/models`, {
                headers: {
                    'Authorization': `Bearer ${this.token}` // Empty bearer for now
                }
            });

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            return await response.json();
        } catch (error) {
            console.error('Failed to fetch models from LiteLLM:', error);
            // Return empty list on error to maintain app stability
            return { data: [], object: 'list' };
        }
    }

    /**
     * Create a chat completion
     * @param request - The completion request
     */
    async createCompletion(request: any): Promise<any> {
        const response = await fetch(`${this.baseURL}/v1/chat/completions`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${this.token}`
            },
            body: JSON.stringify(request)
        });

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        return await response.json();
    }

    /**
     * Set authentication token (for future implementation)
     */
    setToken(token: string): void {
        this.token = token;
    }

    /**
     * Update base URL if needed
     */
    setBaseURL(url: string): void {
        this.baseURL = url;
    }

    /**
     * Get current base URL
     */
    getBaseURL(): string {
        return this.baseURL;
    }
}

// Export singleton instance following STATE.md pattern
export const liteLLMClient = new LiteLLMClient();