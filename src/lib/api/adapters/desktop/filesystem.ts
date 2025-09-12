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

	async readFileRaw(path: string): Promise<string> {
		try {
			// Try native Tauri asset protocol first for PDFs
			if (path.endsWith('.pdf')) {
				try {
					const { convertFileSrc } = await import('@tauri-apps/api/core');
					const absolutePath = await invoke<string>('get_absolute_path', { 
						relativePath: path 
					});
					const assetUrl = convertFileSrc(absolutePath);
					console.log(`DesktopFileSystem: Using native asset URL for PDF: ${assetUrl}`);
					// Add cache busting for PDFs to ensure fresh loads
					return `${assetUrl}?t=${Date.now()}`;
				} catch (error) {
					console.warn('DesktopFileSystem: convertFileSrc failed, falling back to data URL:', error);
					// Fall through to existing implementation
				}
			}
			
			// Existing implementation as fallback
			interface FileContentRaw {
				path: string;
				content: string; // base64 encoded
				size: number;
				modified: number;
			}

			console.log(`DesktopFileSystem: Reading raw file: ${path}`);
			const result = await invoke<FileContentRaw>('read_file_raw', { path });
			console.log(`DesktopFileSystem: Got file data - size: ${result.size}, content length: ${result.content.length}`);
			
			// Validate result
			if (!result.content || result.content.length === 0) {
				throw new Error(`Empty file content received for ${path}`);
			}

			// Convert base64 to blob URL with better error handling
			let binary: string;
			try {
				binary = atob(result.content);
			} catch (decodeError) {
				console.error(`DesktopFileSystem: Base64 decode failed for ${path}:`, decodeError);
				throw new Error(`Invalid base64 content for ${path}: ${decodeError}`);
			}

			const bytes = new Uint8Array(binary.length);
			for (let i = 0; i < binary.length; i++) {
				bytes[i] = binary.charCodeAt(i);
			}
			
			// For PDF files in Tauri, use data URL instead of blob URL 
			// because WebKit has restrictions on blob URL access
			const mimeType = this.getMimeType(path);
			console.log(`DesktopFileSystem: Creating data URL - size: ${bytes.length}, mime: ${mimeType}`);
			
			if (path.endsWith('.pdf')) {
				// Use data URL for PDFs - more reliable in Tauri WebKit
				const dataUrl = `data:${mimeType};base64,${result.content}`;
				console.log(`DesktopFileSystem: Created data URL for PDF (${dataUrl.length} chars)`);
				return dataUrl;
			} else {
				// Use blob URL for other files
				const blob = new Blob([bytes], { type: mimeType });
				const url = URL.createObjectURL(blob);
				console.log(`DesktopFileSystem: Created blob URL: ${url}`);
				return url;
			}
		} catch (error) {
			console.error(`DesktopFileSystem: Failed to read raw file content for ${path}:`, error);
			throw new Error(`Failed to read raw file content: ${error}`);
		}
	}

	private getMimeType(path: string): string {
		const ext = path.split('.').pop()?.toLowerCase() || '';
		const mimeTypes: Record<string, string> = {
			'pdf': 'application/pdf',
			'png': 'image/png',
			'jpg': 'image/jpeg',
			'jpeg': 'image/jpeg',
			'gif': 'image/gif',
			'svg': 'image/svg+xml',
			'txt': 'text/plain',
			'tex': 'text/plain',
			'log': 'text/plain',
			'aux': 'text/plain',
			'html': 'text/html',
			'css': 'text/css',
			'js': 'application/javascript',
			'json': 'application/json',
		};
		return mimeTypes[ext] || 'application/octet-stream';
	}
}