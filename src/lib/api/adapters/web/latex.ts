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

	async setAutoCompile(enabled: boolean): Promise<void> {
		try {
			await apiClient.post('/api/latex/set-auto-compile', { enabled });
		} catch (error) {
			console.error('Failed to set auto compile:', error);
			throw error;
		}
	}

	async setMainFile(filePath: string | null): Promise<void> {
		try {
			await apiClient.post('/api/latex/set-main-file', { filePath });
		} catch (error) {
			console.error('Failed to set main file:', error);
			throw error;
		}
	}
}