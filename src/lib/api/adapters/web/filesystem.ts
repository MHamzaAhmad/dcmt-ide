// Web File System Operations (HTTP API)
import { apiClient } from '../../client';
import type { FileInfo, FileContent, CreateFileRequest, UpdateFileRequest, FileSystemOperations } from '../../types';

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
				`/api/files/content/${encodeURIComponent(path)}`, 
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
				is_directory: isDirectory 
			};
			const response = await apiClient.post<{ success: boolean; message: string }>(
				'/api/files/create', 
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
				`/api/files/rename/${encodeURIComponent(oldPath)}/${encodeURIComponent(newPath)}`,
				{}
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
			// We can check if a file exists by trying to get its info
			await this.readFileContent(path);
			return true;
		} catch (error) {
			return false;
		}
	}
}