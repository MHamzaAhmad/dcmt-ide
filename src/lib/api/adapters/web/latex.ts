// Web LaTeX Adapter - HTTP API
import { apiClient } from '../../client';
import type { LaTeXCompileRequest, LaTeXCompileResponse, LaTeXOperations } from '../../types';

export class WebLatexAdapter implements LaTeXOperations {
	async compileLatex(request: LaTeXCompileRequest): Promise<LaTeXCompileResponse> {
		try {
			const response = await apiClient.post<LaTeXCompileResponse>('/api/latex/compile', request);
			
			return response;
		} catch (error) {
			console.error('Web LaTeX compilation failed:', error);
			
			// Handle HTTP errors
			const errorMessage = error instanceof Error ? error.message : 'Unknown compilation error';
			
			return {
				success: false,
				message: 'LaTeX compilation failed',
				errors: [errorMessage]
			};
		}
	}

	async findMainLatexFile(): Promise<string> {
		try {
			const response = await apiClient.get<{ success: boolean; main_file?: string; message?: string }>('/api/latex/find-main');
			
			if (!response.success) {
				throw new Error(response.message || 'Failed to find main LaTeX file');
			}
			
			return response.main_file || '';
		} catch (error) {
			console.error('Web LaTeX find main file failed:', error);
			throw error;
		}
	}
}