// Unified API Adapter - Platform Detection and Selection
import { isTauri } from '$lib/utils/platform';
import { DesktopApiAdapter } from './desktop';
import { WebApiAdapter } from './web';
import { WebGitAdapter } from './web/git';
import { DesktopGitAdapter } from './desktop/git';
import type { PlatformAPI, ProjectInfo, LaTeXCompileRequest, LaTeXCompileResponse, GitOperations, CompilationEvent, LatexBuildState } from '../types';

// Create singleton instances lazily
let desktopAdapter: DesktopApiAdapter | null = null;
let webAdapter: WebApiAdapter | null = null;
let desktopGitAdapter: DesktopGitAdapter | null = null;
let webGitAdapter: WebGitAdapter | null = null;

/**
 * Get the appropriate API adapter based on the current platform
 * @returns PlatformAPI adapter for current platform
 */
export function getApiAdapter(): PlatformAPI {
	if (isTauri()) {
		if (!desktopAdapter) {
			desktopAdapter = new DesktopApiAdapter();
		}
		return desktopAdapter;
	} else {
		if (!webAdapter) {
			webAdapter = new WebApiAdapter();
		}
		return webAdapter;
	}
}

/**
 * Get the desktop API adapter (Tauri commands)
 * @returns DesktopApiAdapter instance
 */
export function getDesktopAdapter(): DesktopApiAdapter {
	if (!desktopAdapter) {
		desktopAdapter = new DesktopApiAdapter();
	}
	return desktopAdapter;
}

/**
 * Get the web API adapter (HTTP requests)
 * @returns WebApiAdapter instance
 */
export function getWebAdapter(): WebApiAdapter {
	if (!webAdapter) {
		webAdapter = new WebApiAdapter();
	}
	return webAdapter;
}

/**
 * Unified Platform API - automatically uses correct adapter
 */
export class UnifiedPlatformApi implements PlatformAPI {
	private get adapter(): PlatformAPI {
		return getApiAdapter();
	}

	// Project management (desktop only)
	async selectProjectFolder(): Promise<ProjectInfo | null> {
		return await this.adapter.selectProjectFolder?.() || null;
	}

	async getCurrentProject(): Promise<ProjectInfo | null> {
		return await this.adapter.getCurrentProject?.() || null;
	}

	async clearProject(): Promise<void> {
		return await this.adapter.clearProject?.() || Promise.resolve();
	}

	// File operations
	async getDirectoryTree(path: string = '') {
		return await this.adapter.getDirectoryTree(path);
	}

	async readFileContent(path: string) {
		return await this.adapter.readFileContent(path);
	}

	async readFileRaw(path: string): Promise<string> {
		return await this.adapter.readFileRaw(path);
	}

	async writeFileContent(path: string, content: string) {
		return await this.adapter.writeFileContent(path, content);
	}

	async createFile(path: string, content?: string, isDirectory?: boolean) {
		return await this.adapter.createFile(path, content, isDirectory);
	}

	async deleteFile(path: string) {
		return await this.adapter.deleteFile(path);
	}

	async renameFile(oldPath: string, newPath: string) {
		return await this.adapter.renameFile(oldPath, newPath);
	}

	async fileExists(path: string) {
		return await this.adapter.fileExists(path);
	}

	// LaTeX operations
	async compileLatex(request: LaTeXCompileRequest): Promise<LaTeXCompileResponse> {
		if (!this.adapter.compileLatex) {
			throw new Error('LaTeX compilation not supported on this platform');
		}
		return await this.adapter.compileLatex(request);
	}

	async findMainLatexFile(): Promise<string> {
		if (!this.adapter.findMainLatexFile) {
			throw new Error('LaTeX main file finding not supported on this platform');
		}
		return await this.adapter.findMainLatexFile();
	}

	async getLatexStatus(): Promise<LatexBuildState> {
		if (!this.adapter.getLatexStatus) {
			throw new Error('LaTeX status not supported on this platform');
		}
		return await this.adapter.getLatexStatus();
	}

	// Compilation events
	onCompilationEvent(callback: (event: CompilationEvent) => void): () => void {
		if (!this.adapter.onCompilationEvent) {
			console.warn('Compilation events not supported on this platform');
			return () => {}; // Return no-op unsubscribe function
		}
		return this.adapter.onCompilationEvent(callback);
	}

	async setAutoCompile(enabled: boolean): Promise<void> {
		if (!this.adapter.setAutoCompile) {
			console.warn('Auto-compile setting not supported on this platform');
			return;
		}
		return await this.adapter.setAutoCompile(enabled);
	}

	async setMainFile(filePath: string | null): Promise<void> {
		if (!this.adapter.setMainFile) {
			console.warn('Main file setting not supported on this platform');
			return;
		}
		return await this.adapter.setMainFile(filePath);
	}

	// Platform-specific utilities
	get supportsProjectSelection(): boolean {
		return isTauri();
	}

	get platform(): 'desktop' | 'web' {
		return isTauri() ? 'desktop' : 'web';
	}
}

// Export singleton instance
export const platformApi = new UnifiedPlatformApi();

// Keep backward compatibility
export const fileSystemApi = platformApi;

// Export adapters for direct access if needed
export { DesktopApiAdapter, WebApiAdapter };

/**
 * Get the appropriate Git adapter based on the current platform
 * @returns GitOperations adapter for current platform
 */
export function getGitAdapter(): GitOperations {
	if (isTauri()) {
		if (!desktopGitAdapter) {
			desktopGitAdapter = new DesktopGitAdapter();
		}
		return desktopGitAdapter;
	} else {
		if (!webGitAdapter) {
			webGitAdapter = new WebGitAdapter();
		}
		return webGitAdapter;
	}
}

/**
 * Get the desktop Git adapter (Tauri commands)
 * @returns DesktopGitAdapter instance
 */
export function getDesktopGitAdapter(): DesktopGitAdapter {
	if (!desktopGitAdapter) {
		desktopGitAdapter = new DesktopGitAdapter();
	}
	return desktopGitAdapter;
}

/**
 * Get the web Git adapter (HTTP requests)
 * @returns WebGitAdapter instance
 */
export function getWebGitAdapter(): WebGitAdapter {
	if (!webGitAdapter) {
		webGitAdapter = new WebGitAdapter();
	}
	return webGitAdapter;
}

// Export platform-specific adapters
export * from './desktop';
export * from './web';