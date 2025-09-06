// Desktop LaTeX Adapter - Tauri Commands
import { invoke } from '@tauri-apps/api/core';
import type { LaTeXCompileRequest, LaTeXCompileResponse, LaTeXOperations } from '../../types';

export class DesktopLatexAdapter implements LaTeXOperations {
	async compileLatex(request: LaTeXCompileRequest): Promise<LaTeXCompileResponse> {
		try {
			const response = await invoke<LaTeXCompileResponse>('compile_latex', {
				request
			});
			
			return response;
		} catch (error) {
			console.error('Desktop LaTeX compilation failed:', error);
			
			// Handle Tauri invoke errors
			const errorMessage = error instanceof Error ? error.message : 'Unknown compilation error';
			
			return {
				success: false,
				message: 'LaTeX compilation failed',
				errors: [errorMessage]
			};
		}
	}
}