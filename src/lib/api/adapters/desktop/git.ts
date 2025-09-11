import { invoke } from '@tauri-apps/api/core';
import type {
	GitStatus,
	GitDiff,
	CommitSummary,
	CommitResult,
	GitOperations
} from '../../types';

export class DesktopGitAdapter implements GitOperations {
	private initialized = false;

	async initialize(workspacePath: string, litellmBaseUrl: string): Promise<void> {
		try {
			await invoke('initialize_git_service', {
				workspacePath,
				litellmBaseUrl
			});
			this.initialized = true;
		} catch (error) {
			throw new Error(`Failed to initialize Git service: ${error}`);
		}
	}

	private ensureInitialized(): void {
		if (!this.initialized) {
			throw new Error('Git service not initialized. Call initialize() first.');
		}
	}

	async getStatus(): Promise<GitStatus> {
		this.ensureInitialized();
		
		try {
			return await invoke<GitStatus>('get_git_status');
		} catch (error) {
			throw new Error(`Failed to get git status: ${error}`);
		}
	}

	async getDiff(staged: boolean): Promise<GitDiff> {
		this.ensureInitialized();
		
		try {
			return await invoke<GitDiff>('get_git_diff', { staged });
		} catch (error) {
			throw new Error(`Failed to get git diff: ${error}`);
		}
	}

	async generateSummary(staged: boolean): Promise<CommitSummary> {
		this.ensureInitialized();
		
		try {
			return await invoke<CommitSummary>('generate_commit_summary', { staged });
		} catch (error) {
			throw new Error(`Failed to generate commit summary: ${error}`);
		}
	}

	async stageFiles(paths: string[]): Promise<void> {
		this.ensureInitialized();
		
		try {
			await invoke('stage_files', { paths });
		} catch (error) {
			throw new Error(`Failed to stage files: ${error}`);
		}
	}

	async stageAll(): Promise<void> {
		this.ensureInitialized();
		
		try {
			await invoke('stage_all_files');
		} catch (error) {
			throw new Error(`Failed to stage all files: ${error}`);
		}
	}

	async commit(message: string): Promise<CommitResult> {
		this.ensureInitialized();
		
		if (!message.trim()) {
			throw new Error('Commit message cannot be empty');
		}
		
		try {
			return await invoke<CommitResult>('commit_changes', { message });
		} catch (error) {
			throw new Error(`Failed to commit changes: ${error}`);
		}
	}

	async push(): Promise<void> {
		this.ensureInitialized();
		
		try {
			await invoke('push_changes');
		} catch (error) {
			throw new Error(`Failed to push changes: ${error}`);
		}
	}

	async commitAndPush(message: string): Promise<CommitResult> {
		this.ensureInitialized();
		
		if (!message.trim()) {
			throw new Error('Commit message cannot be empty');
		}
		
		try {
			return await invoke<CommitResult>('commit_and_push_changes', { message });
		} catch (error) {
			throw new Error(`Failed to commit and push changes: ${error}`);
		}
	}

	isInitialized(): boolean {
		return this.initialized;
	}
}