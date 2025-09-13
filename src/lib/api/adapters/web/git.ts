import { apiClient } from '../../client';
import type {
	GitStatus,
	GitDiff,
	CommitSummary,
	CommitResult,
	GitOperations
} from '../../types';

export class WebGitAdapter implements GitOperations {

	async getStatus(): Promise<GitStatus> {
		try {
			const response = await apiClient.get<{ status: GitStatus }>('/api/git/status');
			return response.status;
		} catch (error) {
			console.error('Failed to get git status:', error);
			throw error;
		}
	}

	async getDiff(staged: boolean): Promise<GitDiff> {
		try {
			const params = new URLSearchParams({ staged: staged.toString() });
			const response = await apiClient.get<{ diff: GitDiff }>(`/api/git/diff?${params}`);
			return response.diff;
		} catch (error) {
			console.error('Failed to get git diff:', error);
			throw error;
		}
	}

	async generateSummary(staged: boolean): Promise<CommitSummary> {
		try {
			const params = new URLSearchParams({ staged: staged.toString() });
			const response = await apiClient.post<{ summary: CommitSummary }>(`/api/git/summary?${params}`, {});
			return response.summary;
		} catch (error) {
			console.error('Failed to generate commit summary:', error);
			throw error;
		}
	}

	async stageFiles(paths: string[]): Promise<void> {
		try {
			await apiClient.post('/api/git/stage', { paths });
		} catch (error) {
			console.error('Failed to stage files:', error);
			throw error;
		}
	}

	async stageAll(): Promise<void> {
		try {
			await apiClient.post('/api/git/stage-all', {});
		} catch (error) {
			console.error('Failed to stage all files:', error);
			throw error;
		}
	}

	async commit(message: string): Promise<CommitResult> {
		try {
			const response = await apiClient.post<{ commit: CommitResult }>('/api/git/commit', { message });
			return response.commit;
		} catch (error) {
			console.error('Failed to commit changes:', error);
			throw error;
		}
	}

	async push(): Promise<void> {
		try {
			await apiClient.post('/api/git/push', {});
		} catch (error) {
			console.error('Failed to push changes:', error);
			throw error;
		}
	}

	async commitAndPush(message: string): Promise<CommitResult> {
		try {
			const response = await apiClient.post<{ commit: CommitResult }>('/api/git/commit-and-push', { message });
			return response.commit;
		} catch (error) {
			console.error('Failed to commit and push changes:', error);
			throw error;
		}
	}
}