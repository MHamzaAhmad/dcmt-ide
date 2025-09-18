import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import { getGitAdapter, getDesktopGitAdapter } from '$lib/api/adapters';
import { isTauri } from '$lib/utils/platform';
import { eventStore } from './index';
import { shouldSkipGitRefresh } from '$lib/utils/filePatterns';
import type {
	GitStatus,
	GitDiff,
	CommitSummary,
	CommitResult,
	GitOperations
} from '$lib/api/types';

// ============================================================================
// Git Store State
// ============================================================================

export interface GitStoreState {
	isReady: boolean;
	isInitialized: boolean;
	workspacePath: string | null;
	
	// Git status
	status: GitStatus | null;
	diff: GitDiff | null;
	stagedDiff: GitDiff | null;
	
	// AI summary
	aiSummary: CommitSummary | null;
	isGeneratingSummary: boolean;

	/**
	 * Hash of the unstaged diff used to produce the current aiSummary.
	 * Helps prevent redundant regenerations when nothing has changed.
	 */
	lastSummaryDiffHash?: string | null;

	// AI summary cache & config (unstaged-only flow)
	/**
	 * Cache commit summaries by a stable diff hash to avoid repeated LLM calls.
	 * Key: diffHash (unstaged). Value: { summary, ts }
	 */
	aiSummaryCache?: Map<string, { summary: CommitSummary; ts: number }>;
	/** Enable auto-generation for unstaged changes */
	autoGenerateSummary?: boolean;
	/** TTL for cached summaries (ms) */
	summaryTtlMs?: number;
	
	// Operations state
	isStaging: boolean;
	isCommitting: boolean;
	isPushing: boolean;
	isRefreshing: boolean;
	
	// Results
	lastCommit: CommitResult | null;
	lastError: string | null;
	
	// Configuration
	autoRefreshInterval: number;
	enableAutoRefresh: boolean;
}

// ============================================================================
// Git Store Implementation
// ============================================================================

function createGitStore() {
	const initialState: GitStoreState = {
		isReady: false,
		isInitialized: false,
		workspacePath: null,
		status: null,
		diff: null,
		stagedDiff: null,
		aiSummary: null,
		isGeneratingSummary: false,
		lastSummaryDiffHash: null,
		aiSummaryCache: new Map(),
		autoGenerateSummary: true,
		summaryTtlMs: 15 * 60 * 1000, // 15 minutes
		isStaging: false,
		isCommitting: false,
		isPushing: false,
		isRefreshing: false,
		lastCommit: null,
		lastError: null,
		autoRefreshInterval: 5000, // 5 seconds
		enableAutoRefresh: true,
	};

	const { subscribe, update } = writable<GitStoreState>(initialState);

	// Git adapter instance
	let gitAdapter: GitOperations | null = null;
	let autoRefreshTimer: ReturnType<typeof setInterval> | null = null;
	let summaryDebounceTimer: ReturnType<typeof setTimeout> | null = null;
	const inFlightSummaries = new Set<string>(); // diffHash in flight

	function hashString(input: string): string {
		// Simple, fast non-crypto hash (djb2) suitable for cache keys
		let hash = 5381;
		for (let i = 0; i < input.length; i++) {
			hash = ((hash << 5) + hash) + input.charCodeAt(i);
			hash = hash & 0xffffffff;
		}
		// Convert to unsigned hex string
		return (hash >>> 0).toString(16);
	}

	function canonicalizeDiff(diff: GitDiff): string {
		// Stable string representation: sort files by path, include status, counts and hunks
		const files = [...(diff.files || [])].sort((a, b) => a.path.localeCompare(b.path));
		const parts: string[] = [];
		for (const f of files) {
			parts.push(`PATH:${f.path}`);
			parts.push(`STATUS:${f.status}`);
			parts.push(`ADD:${f.additions}`);
			parts.push(`DEL:${f.deletions}`);
			if (f.hunks && f.hunks.length) {
				// Include limited hunk content to keep key stable yet bounded
				const hunksPreview = f.hunks.slice(0, 20).join('\n');
				parts.push(`HUNKS:${hunksPreview}`);
			}
		}
		// Include overall stats
		if (diff.stats) {
			parts.push(`STATS:${diff.stats.files_changed}:${diff.stats.additions}:${diff.stats.deletions}`);
		}
		return parts.join('|');
	}

	function getUnstagedDiffHash(state: GitStoreState): string | null {
		if (!state.diff || !state.status) return null;
		const hasUnstaged = (state.status.unstaged?.length || 0) > 0 || (state.status.untracked?.length || 0) > 0;
		if (!hasUnstaged) return null;
		const key = canonicalizeDiff(state.diff);
		return hashString(key);
	}

	// ============================================================================
	// Core Methods
	// ============================================================================

	async function initialize(workspacePath: string = '', litellmBaseUrl: string = 'http://localhost:4000'): Promise<void> {
		if (!browser) return;

		try {
			// Clear any previous error
			update(state => ({ ...state, lastError: null }));

			// Get the appropriate adapter
			gitAdapter = getGitAdapter();

			// Initialize desktop adapter if needed
			if (isTauri() && gitAdapter instanceof getDesktopGitAdapter().constructor) {
				const desktopAdapter = gitAdapter as any;
				if (!desktopAdapter.isInitialized()) {
					await desktopAdapter.initialize(workspacePath, litellmBaseUrl);
				}
			}

			// Test the connection by getting status
			await refresh();

			update(state => ({
				...state,
				isReady: true,
				isInitialized: true,
				workspacePath,
			}));

			// Start auto-refresh timer
			startAutoRefresh();

			// Subscribe to file system events for auto-refresh
			subscribeToEvents();

			// Emit initialization event
			eventStore.events.gitStatusChanged('unknown', null);

		} catch (error) {
			const errorMessage = `Failed to initialize Git store: ${error}`;
			console.error('[GitStore]', errorMessage);
			
			update(state => ({
				...state,
				isReady: false,
				isInitialized: false,
				lastError: errorMessage,
			}));
			
			eventStore.events.gitError(errorMessage);
			throw error;
		}
	}

	async function refresh(): Promise<void> {
		if (!gitAdapter) {
			throw new Error('Git adapter not initialized');
		}

		try {
			update(state => ({ ...state, isRefreshing: true, lastError: null }));

			// Get current status
			const status = await gitAdapter.getStatus();
			
			// Get unstaged diff if there are changes
			let diff: GitDiff | null = null;
			let stagedDiff: GitDiff | null = null;
			
			if (status.unstaged.length > 0 || status.untracked.length > 0) {
				diff = await gitAdapter.getDiff(false);
			}
			
			if (status.staged.length > 0) {
				stagedDiff = await gitAdapter.getDiff(true);
			}

			update(state => ({
				...state,
				status,
				diff,
				stagedDiff,
				isRefreshing: false,
			}));

			// Emit status changed event
			eventStore.events.gitStatusChanged(status.branch, status);

			// Clear AI summary if no changes
			if (status.staged.length === 0 && status.unstaged.length === 0 && status.untracked.length === 0) {
				update(state => ({ ...state, aiSummary: null, lastSummaryDiffHash: null }));
			} else {
				// Auto-generate summary for unstaged changes (debounced)
				maybeAutoGenerateSummary();
			}

		} catch (error) {
			const errorMessage = `Failed to refresh Git status: ${error}`;
			console.error('[GitStore]', errorMessage);
			
			update(state => ({
				...state,
				isRefreshing: false,
				lastError: errorMessage,
			}));
			
			eventStore.events.gitError(errorMessage);
			throw error;
		}
	}

	async function generateSummary(useStaged: boolean = false): Promise<void> {
		if (!gitAdapter) {
			throw new Error('Git adapter not initialized');
		}

		try {
			update(state => ({ 
				...state, 
				isGeneratingSummary: true, 
				lastError: null,
				aiSummary: null 
			}));

			const summary = await gitAdapter.generateSummary(useStaged);

			update(state => ({
				...state,
				aiSummary: summary,
				isGeneratingSummary: false,
			}));

			// Emit summary generated event
			eventStore.events.gitSummaryGenerated(summary);

		} catch (error) {
			const errorMessage = `Failed to generate AI summary: ${error}`;
			console.error('[GitStore]', errorMessage);
			
			update(state => ({
				...state,
				isGeneratingSummary: false,
				lastError: errorMessage,
			}));
			
			eventStore.events.gitError(errorMessage);
			throw error;
		}
	}

	// Ensure AI summary for current UNSTAGED diff using cache and debounce
	async function ensureSummary(options?: { force?: boolean }): Promise<void> {
		const state = get({ subscribe });
		if (!state.autoGenerateSummary) return;
		if (!gitAdapter) return;

		const hasUnstaged = state.status && ((state.status.unstaged.length > 0) || (state.status.untracked.length > 0));
		if (!hasUnstaged) return;

		// Need unstaged diff to compute hash; fetch if missing
		let diff = state.diff;
		if (!diff) {
			try {
				diff = await gitAdapter.getDiff(false);
				update(s => ({ ...s, diff }));
			} catch (e) {
				console.error('[GitStore] Failed to fetch unstaged diff for summary:', e);
				return;
			}
		}

		const diffHash = getUnstagedDiffHash(get({ subscribe }));
		if (!diffHash) return;

		const cache = state.aiSummaryCache!;
		const now = Date.now();
		const ttl = state.summaryTtlMs ?? 15 * 60 * 1000;
		const cached = cache.get(diffHash);
		if (cached && !options?.force && (now - cached.ts) < ttl) {
			// Serve from cache and mark current diff hash
			update(s => ({ ...s, aiSummary: cached.summary, lastSummaryDiffHash: diffHash }));
			return;
		}

		if (inFlightSummaries.has(diffHash)) {
			return; // already generating for this hash
		}

		try {
			inFlightSummaries.add(diffHash);
			update(s => ({ ...s, isGeneratingSummary: true, lastError: null }));
			const summary = await gitAdapter.generateSummary(false);
			// Cache and publish
			cache.set(diffHash, { summary, ts: now });
			update(s => ({ ...s, aiSummary: summary, isGeneratingSummary: false, lastSummaryDiffHash: diffHash }));
			eventStore.events.gitSummaryGenerated(summary);
		} catch (error) {
			const errorMessage = `Failed to generate AI summary (unstaged): ${error}`;
			console.error('[GitStore]', errorMessage);
			update(s => ({ ...s, isGeneratingSummary: false, lastError: errorMessage }));
			eventStore.events.gitError(errorMessage);
		} finally {
			inFlightSummaries.delete(diffHash);
		}
	}

	function maybeAutoGenerateSummary() {
		const state = get({ subscribe });
		if (!state.autoGenerateSummary) return;
		if (summaryDebounceTimer) clearTimeout(summaryDebounceTimer);
		summaryDebounceTimer = setTimeout(() => {
			ensureSummary().catch(console.error);
		}, 1000);
	}

	async function stageFiles(paths: string[]): Promise<void> {
		if (!gitAdapter) {
			throw new Error('Git adapter not initialized');
		}

		try {
			update(state => ({ ...state, isStaging: true, lastError: null }));

			await gitAdapter.stageFiles(paths);

			// Emit files staged event
			eventStore.events.gitFilesStaged(paths);

			// Refresh to get updated status
			await refresh();

			update(state => ({ ...state, isStaging: false }));

		} catch (error) {
			const errorMessage = `Failed to stage files: ${error}`;
			console.error('[GitStore]', errorMessage);
			
			update(state => ({
				...state,
				isStaging: false,
				lastError: errorMessage,
			}));
			
			eventStore.events.gitError(errorMessage);
			throw error;
		}
	}

	async function stageAll(): Promise<void> {
		if (!gitAdapter) {
			throw new Error('Git adapter not initialized');
		}

		try {
			update(state => ({ ...state, isStaging: true, lastError: null }));

			await gitAdapter.stageAll();

			const currentState = get({ subscribe });
			const allFiles = [
				...(currentState.status?.unstaged.map(f => f.path) || []),
				...(currentState.status?.untracked || [])
			];

			// Emit files staged event
			eventStore.events.gitFilesStaged(allFiles);

			// Refresh to get updated status
			await refresh();

			update(state => ({ ...state, isStaging: false }));

		} catch (error) {
			const errorMessage = `Failed to stage all files: ${error}`;
			console.error('[GitStore]', errorMessage);
			
			update(state => ({
				...state,
				isStaging: false,
				lastError: errorMessage,
			}));
			
			eventStore.events.gitError(errorMessage);
			throw error;
		}
	}

	async function commit(message: string): Promise<CommitResult> {
		if (!gitAdapter) {
			throw new Error('Git adapter not initialized');
		}

		if (!message.trim()) {
			throw new Error('Commit message cannot be empty');
		}

		try {
			update(state => ({ ...state, isCommitting: true, lastError: null }));

			const result = await gitAdapter.commit(message);

			update(state => ({
				...state,
				lastCommit: result,
				isCommitting: false,
				aiSummary: null, // Clear summary after commit
			}));

			// Emit commit created event
			eventStore.events.gitCommitCreated(result);

			// Refresh to get updated status
			await refresh();

			return result;

		} catch (error) {
			const errorMessage = `Failed to commit changes: ${error}`;
			console.error('[GitStore]', errorMessage);
			
			update(state => ({
				...state,
				isCommitting: false,
				lastError: errorMessage,
			}));
			
			eventStore.events.gitError(errorMessage);
			throw error;
		}
	}

	async function push(): Promise<void> {
		if (!gitAdapter) {
			throw new Error('Git adapter not initialized');
		}

		try {
			update(state => ({ ...state, isPushing: true, lastError: null }));

			await gitAdapter.push();

			// Emit push completed event
			eventStore.events.gitPushCompleted();

			// Refresh to get updated status
			await refresh();

			update(state => ({ ...state, isPushing: false }));

		} catch (error) {
			const errorMessage = `Failed to push changes: ${error}`;
			console.error('[GitStore]', errorMessage);
			
			update(state => ({
				...state,
				isPushing: false,
				lastError: errorMessage,
			}));
			
			eventStore.events.gitError(errorMessage);
			throw error;
		}
	}

	async function commitAndPush(message: string): Promise<CommitResult> {
		if (!gitAdapter) {
			throw new Error('Git adapter not initialized');
		}

		if (!message.trim()) {
			throw new Error('Commit message cannot be empty');
		}

		try {
			update(state => ({ 
				...state, 
				isCommitting: true, 
				isPushing: true, 
				lastError: null 
			}));

			const result = await gitAdapter.commitAndPush(message);

			update(state => ({
				...state,
				lastCommit: result,
				isCommitting: false,
				isPushing: false,
				aiSummary: null, // Clear summary after commit
			}));

			// Emit commit and push events
			eventStore.events.gitCommitCreated(result);
			eventStore.events.gitPushCompleted();

			// Refresh to get updated status
			await refresh();

			return result;

		} catch (error) {
			const errorMessage = `Failed to commit and push changes: ${error}`;
			console.error('[GitStore]', errorMessage);
			
			update(state => ({
				...state,
				isCommitting: false,
				isPushing: false,
				lastError: errorMessage,
			}));
			
			eventStore.events.gitError(errorMessage);
			throw error;
		}
	}

	// ============================================================================
	// Event Subscription
	// ============================================================================

	function subscribeToEvents(): void {
		if (!browser) return;

		// Subscribe to file system events for auto-refresh
		eventStore.fileSystemEvents.subscribe(events => {
			const latestEvent = events[events.length - 1];
			if (latestEvent && get({ subscribe }).enableAutoRefresh) {
				const filePath = latestEvent.payload.path;
				
				// Only refresh git for meaningful file changes, not generated/temporary files
				// This prevents infinite loops with files like .synctex.gz, .aux, .xdv, etc.
				if (shouldSkipGitRefresh(filePath)) {
					console.log(`GitStore: Skipping auto-refresh for generated/temporary file: ${filePath}`);
					return;
				}
				
				console.log(`GitStore: Auto-refreshing due to file change: ${filePath}`);
				// Debounce refresh calls
				if (autoRefreshTimer) {
					clearTimeout(autoRefreshTimer);
				}
				autoRefreshTimer = setTimeout(() => {
					refresh().catch(console.error);
				}, 1000);
			}
		});
	}

	// ============================================================================
	// Auto Refresh
	// ============================================================================

	function startAutoRefresh(): void {
		if (!browser) return;

		const state = get({ subscribe });
		if (!state.enableAutoRefresh) return;

		// Clear existing timer
		if (autoRefreshTimer) {
			clearInterval(autoRefreshTimer);
		}

		// Set up new timer
		autoRefreshTimer = setInterval(() => {
			const currentState = get({ subscribe });
			if (currentState.enableAutoRefresh && !currentState.isRefreshing) {
				refresh().catch(console.error);
			}
		}, state.autoRefreshInterval);
	}

	function stopAutoRefresh(): void {
		if (autoRefreshTimer) {
			clearInterval(autoRefreshTimer);
			autoRefreshTimer = null;
		}
	}

	function setAutoRefresh(enabled: boolean, interval?: number): void {
		update(state => ({
			...state,
			enableAutoRefresh: enabled,
			...(interval && { autoRefreshInterval: interval })
		}));

		if (enabled) {
			startAutoRefresh();
		} else {
			stopAutoRefresh();
		}
	}

	// ============================================================================
	// Derived Properties
	// ============================================================================

	const hasChanges = derived({ subscribe }, $state => 
		$state.status && (
			$state.status.staged.length > 0 || 
			$state.status.unstaged.length > 0 || 
			$state.status.untracked.length > 0
		)
	);

	const hasStagedChanges = derived({ subscribe }, $state => 
		$state.status && $state.status.staged.length > 0
	);

	const hasUnstagedChanges = derived({ subscribe }, $state => 
		$state.status && (
			$state.status.unstaged.length > 0 || 
			$state.status.untracked.length > 0
		)
	);

	const canCommit = derived({ subscribe }, $state => 
		$state.isReady && 
		$state.status && 
		$state.status.staged.length > 0 && 
		!$state.isCommitting
	);

	const canPush = derived({ subscribe }, $state => 
		$state.isReady && 
		$state.status && 
		$state.status.ahead > 0 && 
		!$state.isPushing
	);

	const isLoading = derived({ subscribe }, $state => 
		$state.isRefreshing || 
		$state.isGeneratingSummary || 
		$state.isStaging || 
		$state.isCommitting || 
		$state.isPushing
	);

	// Whether clicking Regenerate would actually do work
	const canRegenerateSummary = derived({ subscribe }, $state => {
		// Need unstaged changes and a current diff
		if (!$state || !$state.status) return false;
		const hasUnstaged = ($state.status.unstaged.length > 0) || ($state.status.untracked.length > 0);
		if (!hasUnstaged) return false;
		if ($state.isGeneratingSummary) return false;
		// If no summary yet, we could generate, but the UI auto-generates; treat as disabled for explicit Regenerate
		if (!$state.aiSummary) return false;
		// If we don't have a diff yet, be conservative and allow regeneration
		if (!$state.diff) return true;
		// Compute current diff hash and compare to lastSummaryDiffHash
		const files = [...($state.diff.files || [])].sort((a, b) => a.path.localeCompare(b.path));
		const parts: string[] = [];
		for (const f of files) {
			parts.push(`PATH:${f.path}`);
			parts.push(`STATUS:${f.status}`);
			parts.push(`ADD:${f.additions}`);
			parts.push(`DEL:${f.deletions}`);
			if (f.hunks && f.hunks.length) {
				const hunksPreview = f.hunks.slice(0, 20).join('\n');
				parts.push(`HUNKS:${hunksPreview}`);
			}
		}
		if ($state.diff.stats) {
			parts.push(`STATS:${$state.diff.stats.files_changed}:${$state.diff.stats.additions}:${$state.diff.stats.deletions}`);
		}
		let hash = 5381;
		const key = parts.join('|');
		for (let i = 0; i < key.length; i++) {
			hash = ((hash << 5) + hash) + key.charCodeAt(i);
			hash = hash & 0xffffffff;
		}
		const currentHash = (hash >>> 0).toString(16);
		return currentHash !== ($state.lastSummaryDiffHash || null);
	});

	// ============================================================================
	// Cleanup
	// ============================================================================

	function cleanup(): void {
		stopAutoRefresh();
		gitAdapter = null;
	}

	// Cleanup on page unload
	if (browser) {
		window.addEventListener('beforeunload', cleanup);
	}

	// ============================================================================
	// Return Store Interface
	// ============================================================================

	return {
		// Core store
		subscribe,
		
		// State management
		initialize,
		cleanup,
		
		// Git operations
		refresh,
		generateSummary,
		ensureSummary,
		stageFiles,
		stageAll,
		commit,
		push,
		commitAndPush,
		
		// Configuration
		setAutoRefresh,
		
		// Derived properties
		hasChanges,
		hasStagedChanges,
		hasUnstagedChanges,
		canCommit,
		canPush,
		isLoading,
		canRegenerateSummary,

		// State access
		getCurrentState(): GitStoreState {
			return get({ subscribe });
		}
	};
}

// ============================================================================
// Export Store Instance
// ============================================================================

export const gitStore = createGitStore();

