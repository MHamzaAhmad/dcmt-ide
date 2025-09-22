// Web File System Operations (HTTP API)
import { apiClient } from '../../client';
import type { FileInfo, FileContent, CreateFileRequest, UpdateFileRequest, FileSystemOperations } from '../../types';
import { getApiBaseUrl } from '$lib/utils/api';

export class WebFileSystemAdapter implements FileSystemOperations {
	// File operations
	async getDirectoryTree(path: string = ''): Promise<FileInfo> {
		try {
			const endpoint = path ? `/api/files/tree/${encodeURIComponent(path)}` : '/api/files/tree';
			const response = await apiClient.get<{ success: boolean; data: FileInfo }>(endpoint);
			
			if (!response.success) {
				throw new Error('API returned success: false');
			}
			
			return response.data;
		} catch (error) {
			console.error('Failed to get directory tree:', error);
			throw new Error(`Failed to get directory tree: ${error}`);
		}
	}

	async readFileContent(path: string): Promise<FileContent> {
		try {
			const response = await apiClient.get<{ success: boolean; data: FileContent }>(
				`/api/files/content/${encodeURIComponent(path)}`
			);
			
			if (!response.success) {
				throw new Error('API returned success: false');
			}
			
			return response.data;
		} catch (error) {
			console.error('Failed to read file content:', error);
			throw new Error(`Failed to read file content: ${error}`);
		}
	}

	async writeFileContent(path: string, content: string): Promise<void> {
		try {
			const request: UpdateFileRequest = { content };
			const response = await apiClient.put<{ success: boolean; message: string }>(
				`/api/files/${encodeURIComponent(path)}`, 
				request
			);
			
			if (!response.success) {
				throw new Error(response.message || 'API returned success: false');
			}
		} catch (error) {
			console.error('Failed to write file content:', error);
			throw new Error(`Failed to write file content: ${error}`);
		}
	}

	async createFile(path: string, content?: string, isDirectory: boolean = false): Promise<void> {
		try {
			const request: CreateFileRequest = { 
				path, 
				content, 
				is_dir: isDirectory 
			};
			const response = await apiClient.post<{ success: boolean; message: string }>(
				'/api/files', 
				request
			);
			
			if (!response.success) {
				throw new Error(response.message || 'API returned success: false');
			}
		} catch (error) {
			console.error('Failed to create file:', error);
			throw new Error(`Failed to create file: ${error}`);
		}
	}

	async deleteFile(path: string): Promise<void> {
		try {
			const response = await apiClient.delete<{ success: boolean; message: string }>(
				`/api/files/${encodeURIComponent(path)}`
			);
			
			if (!response.success) {
				throw new Error(response.message || 'API returned success: false');
			}
		} catch (error) {
			console.error('Failed to delete file:', error);
			throw new Error(`Failed to delete file: ${error}`);
		}
	}

	async renameFile(oldPath: string, newPath: string): Promise<void> {
		try {
			const response = await apiClient.post<{ success: boolean; message: string }>(
				'/api/files/rename',
				{ old_path: oldPath, new_path: newPath }
			);
			
			if (!response.success) {
				throw new Error(response.message || 'API returned success: false');
			}
		} catch (error) {
			console.error('Failed to rename file:', error);
			throw new Error(`Failed to rename file: ${error}`);
		}
	}

	async fileExists(path: string): Promise<boolean> {
		try {
			// For binary files (like PDFs), use HEAD request to /api/files/raw/ endpoint
			const baseUrl = getApiBaseUrl();
			const url = `${baseUrl}/api/files/raw/${encodeURIComponent(path)}`;
			
			const response = await fetch(url, { method: 'HEAD' });
			return response.ok;
		} catch (error) {
			// File doesn't exist or other error - return false without logging
			return false;
		}
	}

	async readFileRaw(path: string): Promise<string> {
		const baseUrl = getApiBaseUrl();
		const url = `${baseUrl}/api/files/raw/${encodeURIComponent(path)}?t=${Date.now()}`;
		return url;
	}
}