/**
 * API utility functions
 */

/**
 * Gets the API base URL from the current hostname
 * Uses the same protocol and hostname as the current page
 * @returns The API base URL string
 */
export function getApiBaseUrl(): string {
	if (typeof window === 'undefined') {
		// During SSR or in Node.js environment, return a default
		return 'http://localhost:3001';
	}
	
	const protocol = window.location.protocol;
	const hostname = window.location.hostname;
	const port = window.location.port;
	
	// If we're on localhost, use port 3001 for the API
	if (hostname === 'localhost' || hostname === '127.0.0.1') {
		return `${protocol}//${hostname}:3001`;
	}
	
	// For production domains, use the same protocol and hostname
	return `${protocol}//${hostname}${port ? `:${port}` : ''}`;
}

/**
 * Gets the WebSocket URL from the current hostname
 * Converts http to ws and https to wss
 * @returns The WebSocket base URL string
 */
export function getWebSocketUrl(): string {
	if (typeof window === 'undefined') {
		return 'ws://localhost:3001';
	}
	
	const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
	const hostname = window.location.hostname;
	const port = window.location.port;
	
	// If we're on localhost, use port 3001 for the API
	if (hostname === 'localhost' || hostname === '127.0.0.1') {
		return `${protocol}//${hostname}:3001`;
	}
	
	// For production domains, use the same hostname
	return `${protocol}//${hostname}${port ? `:${port}` : ''}`;
}