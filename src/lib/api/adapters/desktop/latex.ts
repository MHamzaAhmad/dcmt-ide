// Desktop LaTeX Adapter - Tauri Commands
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { LaTeXCompileRequest, LaTeXCompileResponse, LaTeXOperations, CompilationEvent, LatexBuildState } from '../../types';

export class DesktopLatexAdapter implements LaTeXOperations {
	private compilationEventCallbacks: ((event: CompilationEvent) => void)[] = [];
	private eventUnlistener: UnlistenFn | null = null;
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

	async findMainLatexFile(): Promise<string> {
		try {
			const response = await invoke<string>('find_main_latex_file');
			return response;
		} catch (error) {
			console.error('Desktop LaTeX find main file failed:', error);
			throw error;
		}
	}

	// Compilation event handling
	private async initEventListener(): Promise<void> {
		if (this.eventUnlistener) return; // Already listening

		try {
			this.eventUnlistener = await listen<CompilationEvent>('compilation-event', (event) => {
				console.log('Received Tauri compilation event:', event.payload);
				this.compilationEventCallbacks.forEach(callback => {
					try {
						callback(event.payload);
					} catch (error) {
						console.error('Error in compilation event callback:', error);
					}
				});
			});
		} catch (error) {
			console.error('Failed to listen for Tauri compilation events:', error);
		}
	}

	onCompilationEvent(callback: (event: CompilationEvent) => void): () => void {
		this.compilationEventCallbacks.push(callback);
		
		// Initialize event listener if this is the first callback
		if (this.compilationEventCallbacks.length === 1) {
			this.initEventListener();
		}
		
		return () => {
			const index = this.compilationEventCallbacks.indexOf(callback);
			if (index > -1) {
				this.compilationEventCallbacks.splice(index, 1);
			}
			
			// Clean up event listener if no more callbacks
			if (this.compilationEventCallbacks.length === 0 && this.eventUnlistener) {
				this.eventUnlistener();
				this.eventUnlistener = null;
			}
		};
	}

	async setAutoCompile(enabled: boolean): Promise<void> {
		try {
			await invoke('set_auto_compile', { enabled });
		} catch (error) {
			console.error('Failed to set auto compile:', error);
			throw error;
		}
	}

	async setMainFile(filePath: string | null): Promise<void> {
		try {
			await invoke('set_main_file', { filePath });
		} catch (error) {
			console.error('Failed to set main file:', error);
			throw error;
		}
	}

	async getLatexStatus(): Promise<LatexBuildState> {
		try {
			const state = await invoke<LatexBuildState>('get_latex_status');
			return state;
		} catch (error) {
			console.error('Failed to get LaTeX status:', error);
			// Fallback to a minimal idle state
			return {
				main_file: null,
				phase: 'idle',
				pdf_path: null,
				pdf_version: 0,
				engine: null,
				errors: null,
				started_at: null,
				finished_at: null,
				session_id: 'desktop',
			};
		}
	}
}