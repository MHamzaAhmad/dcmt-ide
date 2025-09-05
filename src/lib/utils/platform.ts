// Platform Detection Utilities

declare global {
	interface Window {
		__TAURI__?: any;
		__TAURI_INTERNALS__?: any;
	}
}

/**
 * Detect if the application is running in Tauri (desktop) environment
 * @returns true if running in Tauri, false if running in web browser
 */
export function isTauri(): boolean {
	return typeof window !== 'undefined' && window.__TAURI__ !== undefined;
}

/**
 * Detect if the application is running in web browser
 * @returns true if running in web browser, false if running in Tauri
 */
export function isWeb(): boolean {
	return !isTauri();
}

/**
 * Get the current platform type
 * @returns 'desktop' for Tauri, 'web' for browser
 */
export function getPlatform(): 'desktop' | 'web' {
	return isTauri() ? 'desktop' : 'web';
}

/**
 * Get platform-specific configuration
 */
export function getPlatformConfig() {
	return {
		platform: getPlatform(),
		isTauri: isTauri(),
		isWeb: isWeb(),
		supportsFileSystemAccess: isTauri(),
		supportsProjectSelection: isTauri(),
		needsWorkspaceSetup: isTauri(),
	};
}

/**
 * Platform-specific feature detection
 */
export const platformFeatures = {
	fileSystemAccess: isTauri(),
	projectSelection: isTauri(),
	fileWatcher: isTauri(),
	nativeDialogs: isTauri(),
	webSocketRequired: isWeb(),
	httpApiRequired: isWeb(),
};