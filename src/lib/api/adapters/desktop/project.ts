// Desktop Project Management Operations
import { invoke } from '@tauri-apps/api/core';
import type { ProjectInfo, ProjectOperations } from '../../types';

export class DesktopProjectAdapter implements ProjectOperations {
	// Project management
	async selectProjectFolder(): Promise<ProjectInfo | null> {
		try {
			console.log('Calling select_project_folder Tauri command');
			const result = await invoke<ProjectInfo | null>('select_project_folder');
			console.log('select_project_folder result:', result);
			return result;
		} catch (error) {
			console.error('Failed to select project folder:', error);
			throw new Error(`Failed to select project folder: ${error}`);
		}
	}

	async getCurrentProject(): Promise<ProjectInfo | null> {
		try {
			const result = await invoke<ProjectInfo | null>('get_current_project');
			return result;
		} catch (error) {
			console.error('Failed to get current project:', error);
			return null;
		}
	}

	async clearProject(): Promise<void> {
		try {
			await invoke('clear_project');
		} catch (error) {
			console.error('Failed to clear project:', error);
			throw new Error(`Failed to clear project: ${error}`);
		}
	}

	// Helper method to get project info
	async getProjectInfo(): Promise<Record<string, string>> {
		try {
			const result = await invoke<Record<string, string>>('get_project_info');
			return result;
		} catch (error) {
			console.error('Failed to get project info:', error);
			return { has_project: 'false' };
		}
	}
}