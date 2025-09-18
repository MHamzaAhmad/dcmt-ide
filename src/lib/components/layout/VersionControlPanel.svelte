<script lang="ts">
import { GitBranch, Upload, Loader2, AlertCircle, Check, GitCommit, RefreshCw } from '@lucide/svelte';
	import { gitStore } from '$lib/stores';
	import { onMount } from 'svelte';

	// Access derived properties directly from gitStore
	const { hasChanges, hasUnstagedChanges, canRegenerateSummary } = gitStore;

	let commitMessage = '';
	let isExpanded = false;

	onMount(() => {
		// Trigger ensureSummary for unstaged on panel mount
		gitStore.ensureSummary().catch(console.error);
	});

	async function handleRegenerateSummary() {
		try {
			// Avoid forced regeneration if nothing changed
			if (!$canRegenerateSummary) return;
			await gitStore.ensureSummary({ force: true });
		} catch (error) {
			console.error('Failed to regenerate summary:', error);
		}
	}

	async function handlePublish() {
		// Use AI suggestion if user didn't type a message
		const state = gitStore.getCurrentState();
		const autoMessage = commitMessage.trim() || state.aiSummary?.suggestedMessage || state.aiSummary?.summary || '';
		if (!autoMessage) return;

		try {
			// Stage all implicitly, then commit & push
			await gitStore.stageAll();
			await gitStore.commitAndPush(autoMessage);
			commitMessage = '';
			isExpanded = false;
		} catch (error) {
			console.error('Failed to publish changes:', error);
		}
	}

	async function handleRefresh() {
		try {
			await gitStore.refresh();
			await gitStore.ensureSummary();
		} catch (error) {
			console.error('Failed to refresh git status:', error);
		}
	}
</script>

<div class="h-full flex flex-col">
	<div class="px-3 py-1 border-b flex items-center justify-between">
		<h3 class="text-sm font-medium">Version Control</h3>
		{#if $gitStore.isReady}
			<button 
				on:click={handleRefresh}
				class="p-1 hover:bg-muted rounded-sm transition-colors"
				disabled={$gitStore.isRefreshing}
				title="Refresh"
			>
<RefreshCw size={14} class="text-muted-foreground {$gitStore.isRefreshing ? 'animate-spin' : ''}" />
			</button>
		{/if}
	</div>
	
	{#if !$gitStore.isReady}
		<div class="flex-1 flex items-center justify-center p-6">
			<div class="text-center space-y-4 max-w-xs">
				<div class="flex items-center justify-center">
					<Loader2 size={32} class="text-muted-foreground animate-spin" />
				</div>
				<p class="text-sm text-muted-foreground">
					Initializing Git...
				</p>
			</div>
		</div>
	{:else if $gitStore.lastError}
		<div class="flex-1 flex items-center justify-center p-6">
			<div class="text-center space-y-4 max-w-xs">
				<AlertCircle size={32} class="text-destructive mx-auto" />
				<div class="space-y-2">
					<h4 class="font-medium text-foreground">Git Error</h4>
					<p class="text-sm text-muted-foreground">
						{$gitStore.lastError}
					</p>
				</div>
				<button 
					on:click={handleRefresh}
					class="text-sm text-primary hover:underline"
				>
					Try again
				</button>
			</div>
		</div>
	{:else}
		<div class="flex-1 flex flex-col overflow-hidden">
			<!-- Current Branch -->
			<div class="p-3 border-b">
				<div class="flex items-center gap-2">
					<GitBranch size={16} class="text-muted-foreground" />
					<span class="text-sm font-mono">
						{$gitStore.status?.branch || 'unknown'}
					</span>
				</div>
			</div>

{#if !$hasChanges}
				<!-- No Changes -->
				<div class="flex-1 flex items-center justify-center p-6">
					<div class="text-center space-y-4">
						<Check size={32} class="text-green-500 mx-auto" />
						<div class="space-y-2">
							<h4 class="font-medium text-foreground">All Clean</h4>
							<p class="text-sm text-muted-foreground">
								No changes to commit.
							</p>
						</div>
					</div>
				</div>
			{:else}
				<!-- Changes Available -->
				<div class="flex-1 flex flex-col">
					<!-- Status Summary -->
					<div class="p-3 border-b space-y-2">
						{#if $gitStore.status}
							<div class="text-sm space-y-1">
								{#if $gitStore.status.staged.length > 0}
									<div class="text-green-600">
										• {$gitStore.status.staged.length} staged for commit
									</div>
								{/if}
								{#if $gitStore.status.unstaged.length > 0}
									<div class="text-orange-600">
										• {$gitStore.status.unstaged.length} modified
									</div>
								{/if}
								{#if $gitStore.status.untracked.length > 0}
									<div class="text-blue-600">
										• {$gitStore.status.untracked.length} untracked
									</div>
								{/if}
							</div>
						{/if}

						<!-- Unstaged-only flow: no Stage All button in UI; staging happens on Publish -->
					</div>

{#if $hasUnstagedChanges}
						<!-- AI Summary Section -->
						<div class="p-3 border-b space-y-3">
							<div class="flex items-center justify-between">
								<h4 class="text-sm font-medium">AI Summary</h4>
								<button
									on:click={handleRegenerateSummary}
									disabled={$gitStore.isGeneratingSummary || !$canRegenerateSummary}
									title={$gitStore.isGeneratingSummary
										? 'Generating summary…'
										: (!$canRegenerateSummary
											? 'No changes since last summary'
											: 'Regenerate summary')}
									class="text-xs text-primary hover:underline disabled:opacity-50"
								>
									{#if $gitStore.isGeneratingSummary}
										<Loader2 size={12} class="inline animate-spin mr-1" />
										Generating...
									{:else}
										Regenerate
									{/if}
								</button>
							</div>

							{#if $gitStore.aiSummary}
								<div class="space-y-2">
									<div class="text-sm font-medium">
{$gitStore.aiSummary.summary}
									</div>
									<div class="text-sm text-muted-foreground space-y-1">
{#each $gitStore.aiSummary.bullets as point}
											<div>• {point}</div>
										{/each}
									</div>
								</div>
							{:else if !$gitStore.isGeneratingSummary}
								<p class="text-sm text-muted-foreground">
									Summary will be generated automatically for unstaged changes.
								</p>
							{/if}
						</div>

						<!-- Commit Section -->
						<div class="p-3 space-y-3">
							<button
								on:click={() => isExpanded = !isExpanded}
								class="w-full flex items-center justify-between text-sm font-medium"
							>
								<span>Publish Changes</span>
								<GitCommit size={14} />
							</button>

							{#if isExpanded}
								<div class="space-y-3">
									<textarea
										bind:value={commitMessage}
placeholder={$gitStore.aiSummary?.suggestedMessage || $gitStore.aiSummary?.summary || "Enter commit message..."}
										class="w-full p-2 text-sm border rounded-md resize-none bg-background"
										rows="3"
									></textarea>
									
									<div class="flex gap-2">
										<button
											on:click={() => isExpanded = false}
											class="flex-1 px-3 py-2 text-sm border rounded-md hover:bg-muted transition-colors"
										>
											Cancel
										</button>
										<button
											on:click={handlePublish}
											disabled={($gitStore.isStaging || $gitStore.isCommitting || $gitStore.isPushing) || (!commitMessage.trim() && !$gitStore.aiSummary)}
											class="flex-1 px-3 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50 transition-colors"
										>
											{#if $gitStore.isStaging || $gitStore.isCommitting || $gitStore.isPushing}
												<Loader2 size={14} class="inline animate-spin mr-2" />
												Publishing...
											{:else}
												<Upload size={14} class="inline mr-2" />
												Publish
											{/if}
										</button>
									</div>
								</div>
							{:else}
								<button
									on:click={() => {
										if ($gitStore.aiSummary?.suggestedMessage) {
											commitMessage = $gitStore.aiSummary.suggestedMessage;
										} else if ($gitStore.aiSummary?.summary) {
											commitMessage = $gitStore.aiSummary.summary;
										}
										isExpanded = true;
									}}
disabled={!$hasUnstagedChanges}
									class="w-full px-3 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50 transition-colors"
								>
									<Upload size={14} class="inline mr-2" />
									Publish
								</button>
							{/if}
						</div>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>