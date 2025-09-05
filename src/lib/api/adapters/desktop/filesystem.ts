// Desktop File System Operations
import { invoke } from '@tauri-apps/api/core';
import type { FileInfo, FileContent, FileSystemOperations } from '../../types';

export class DesktopFileSystemAdapter implements FileSystemOperations {
	// File operations
	async getDirectoryTree(path: string = ''): Promise<FileInfo> {
		try {
			const result = await invoke<FileInfo>('get_directory_tree', { path });
			return result;
		} catch (error) {
			console.error('Failed to get directory tree:', error);
			throw new Error(`Failed to get directory tree: ${error}`);
		}
	}

	async readFileContent(path: string): Promise<FileContent> {
		try {
			const result = await invoke<FileContent>('read_file_content', { path });
			return result;
		} catch (error) {
			console.error('Failed to read file content:', error);
			throw new Error(`Failed to read file content: ${error}`);
		}
	}

	async writeFileContent(path: string, content: string): Promise<void> {
		try {
			await invoke('write_file_content', { path, content });
		} catch (error) {
			console.error('Failed to write file content:', error);
			throw new Error(`Failed to write file content: ${error}`);
		}
	}

	async createFile(path: string, content?: string, isDirectory: boolean = false): Promise<void> {
		try {
			await invoke('create_file', { 
				path, 
				content: content || null, 
				isDirectory 
			});
		} catch (error) {
			console.error('Failed to create file:', error);
			throw new Error(`Failed to create file: ${error}`);
		}
	}

	async deleteFile(path: string): Promise<void> {
		try {
			await invoke('delete_file', { path });
		} catch (error) {
			console.error('Failed to delete file:', error);
			throw new Error(`Failed to delete file: ${error}`);
		}
	}

	async renameFile(oldPath: string, newPath: string): Promise<void> {
		try {
			await invoke('rename_file', { oldPath, newPath });
		} catch (error) {
			console.error('Failed to rename file:', error);
			throw new Error(`Failed to rename file: ${error}`);
		}
	}

	async fileExists(path: string): Promise<boolean> {
		try {
			const result = await invoke<boolean>('file_exists', { path });
			return result;
		} catch (error) {
			console.error('Failed to check file existence:', error);
			return false;
		}
	}
}