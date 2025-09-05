// Unified API Adapter - Platform Detection and Selection
import { isTauri } from '$lib/utils/platform';
import { DesktopApiAdapter } from './desktop';
import { WebApiAdapter } from './web';
import type { PlatformAPI, ProjectInfo } from '../types';

// Create singleton instances lazily
let desktopAdapter: DesktopApiAdapter | null = null;
let webAdapter: WebApiAdapter | null = null;

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

// Export platform-specific adapters
export * from './desktop';
export * from './web';