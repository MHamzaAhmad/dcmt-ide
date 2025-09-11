import type {
	GitStatus,
	GitDiff,
	CommitSummary,
	CommitResult,
	GitOperations
} from '../../types';

export class WebGitAdapter implements GitOperations {
	private baseUrl: string;

	constructor(baseUrl: string = 'http://localhost:3001') {
		this.baseUrl = baseUrl;
	}

	async getStatus(): Promise<GitStatus> {
		const response = await fetch(`${this.baseUrl}/api/git/status`, {
			method: 'GET',
			headers: {
				'Content-Type': 'application/json',
			},
		});

		if (!response.ok) {
			const error = await response.json();
			throw new Error(error.error || 'Failed to get git status');
		}

		const data = await response.json();
		return data.status;
	}

	async getDiff(staged: boolean): Promise<GitDiff> {
		const params = new URLSearchParams({ staged: staged.toString() });
		const response = await fetch(`${this.baseUrl}/api/git/diff?${params}`, {
			method: 'GET',
			headers: {
				'Content-Type': 'application/json',
			},
		});

		if (!response.ok) {
			const error = await response.json();
			throw new Error(error.error || 'Failed to get git diff');
		}

		const data = await response.json();
		return data.diff;
	}

	async generateSummary(staged: boolean): Promise<CommitSummary> {
		const params = new URLSearchParams({ staged: staged.toString() });
		const response = await fetch(`${this.baseUrl}/api/git/summary?${params}`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
		});

		if (!response.ok) {
			const error = await response.json();
			throw new Error(error.error || 'Failed to generate commit summary');
		}

		const data = await response.json();
		return data.summary;
	}

	async stageFiles(paths: string[]): Promise<void> {
		const response = await fetch(`${this.baseUrl}/api/git/stage`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify({ paths }),
		});

		if (!response.ok) {
			const error = await response.json();
			throw new Error(error.error || 'Failed to stage files');
		}
	}

	async stageAll(): Promise<void> {
		const response = await fetch(`${this.baseUrl}/api/git/stage-all`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
		});

		if (!response.ok) {
			const error = await response.json();
			throw new Error(error.error || 'Failed to stage all files');
		}
	}

	async commit(message: string): Promise<CommitResult> {
		const response = await fetch(`${this.baseUrl}/api/git/commit`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify({ message }),
		});

		if (!response.ok) {
			const error = await response.json();
			throw new Error(error.error || 'Failed to commit changes');
		}

		const data = await response.json();
		return data.commit;
	}

	async push(): Promise<void> {
		const response = await fetch(`${this.baseUrl}/api/git/push`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
		});

		if (!response.ok) {
			const error = await response.json();
			throw new Error(error.error || 'Failed to push changes');
		}
	}

	async commitAndPush(message: string): Promise<CommitResult> {
		const response = await fetch(`${this.baseUrl}/api/git/commit-and-push`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
			},
			body: JSON.stringify({ message }),
		});

		if (!response.ok) {
			const error = await response.json();
			throw new Error(error.error || 'Failed to commit and push changes');
		}

		const data = await response.json();
		return data.commit;
	}
}