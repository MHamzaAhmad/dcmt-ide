// Web LaTeX Adapter - HTTP API
import { apiClient } from '../../client';
import type { LaTeXCompileRequest, LaTeXCompileResponse, LaTeXOperations } from '../../types';

export class WebLatexAdapter implements LaTeXOperations {
	async compileLatex(request: LaTeXCompileRequest): Promise<LaTeXCompileResponse> {
		try {
			const response = await apiClient.post<LaTeXCompileResponse>('/latex/compile', request);
			
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
}