// Centralized LiteLLM API Client
// Following STATE.md principles: Single Source of Truth for LiteLLM operations

import { authStore } from '$lib/stores/auth';
import type { LiteLLMModelsResponse } from './types';

/**
 * Centralized client for all LiteLLM API interactions
 * Provides a single source of truth for LiteLLM operations across web and desktop platforms
 */
class LiteLLMClient {
    private baseURL: string;

    constructor() {
        // Use relative path for production, fall back to localhost for development
        this.baseURL = import.meta.env.VITE_LITELLM_BASE_URL || '/llm';
    }

    /**
     * Fetch available models from LiteLLM
     */
    async fetchModels(): Promise<LiteLLMModelsResponse> {
        try {
            // Get fresh token from authStore
            const token = await authStore.getToken();
            const headers: Record<string, string> = {};

            if (token) {
                headers['Authorization'] = `Bearer ${token}`;
            }

            const response = await fetch(`${this.baseURL}/v1/models`, {
                headers
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
        // Get fresh token from authStore
        const token = await authStore.getToken();
        const headers: Record<string, string> = {
            'Content-Type': 'application/json'
        };

        if (token) {
            headers['Authorization'] = `Bearer ${token}`;
        }

        const response = await fetch(`${this.baseURL}/v1/chat/completions`, {
            method: 'POST',
            headers,
            body: JSON.stringify(request)
        });

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        return await response.json();
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