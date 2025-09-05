// Web Project Management (Limited - No Folder Selection)
import type { ProjectInfo, ProjectOperations } from '../../types';

export class WebProjectAdapter implements ProjectOperations {
	// Project management - not supported in web mode
	async selectProjectFolder(): Promise<ProjectInfo | null> {
		console.warn('Project folder selection not supported in web mode');
		return null;
	}

	async getCurrentProject(): Promise<ProjectInfo | null> {
		console.warn('Project management not supported in web mode');
		// In web mode, we could return a static project info for /workspace
		return {
			name: 'Workspace',
			path: '/workspace',
			selected_at: Date.now()
		};
	}

	async clearProject(): Promise<void> {
		console.warn('Project management not supported in web mode');
	}

	// Helper method - returns workspace info for web mode
	async getProjectInfo(): Promise<Record<string, string>> {
		return {
			has_project: 'true',
			project_name: 'Workspace',
			project_path: '/workspace',
			platform: 'web'
		};
	}
}