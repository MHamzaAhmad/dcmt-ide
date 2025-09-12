<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { ChevronDown, ChevronRight, Copy, AlertTriangle } from '@lucide/svelte';
	import { latexStore } from '$lib/stores';

	// Reactive store subscriptions
	const latexState = $derived($latexStore);
	
	// Panel state
	let isExpanded = $state(true);
	let copySuccess = $state(false);

	// Derived error information
	const hasErrors = $derived((latexState.lastCompilation?.errors?.length || 0) > 0);
	const errorText = $derived(
		hasErrors ? latexState.lastCompilation?.errors?.join('\n') || '' : ''
	);
	const errorCount = $derived(latexState.lastCompilation?.errors?.length || 0);

	// Copy error text to clipboard
	async function copyErrors() {
		if (!errorText) return;
		
		try {
			await navigator.clipboard.writeText(errorText);
			copySuccess = true;
			setTimeout(() => {
				copySuccess = false;
			}, 2000);
		} catch (err) {
			console.error('Failed to copy errors:', err);
		}
	}

	// Toggle panel expansion
	function toggleExpanded() {
		isExpanded = !isExpanded;
	}
</script>

{#if hasErrors}
<div class="border-t bg-background">
	<!-- Error Panel Header -->
	<div class="flex items-center justify-between px-3 py-2 bg-red-50 border-b border-red-200">
		<button
			onclick={toggleExpanded}
			class="flex items-center gap-2 text-red-800 hover:text-red-900 text-sm font-medium"
		>
			{#if isExpanded}
				<ChevronDown size={16} />
			{:else}
				<ChevronRight size={16} />
			{/if}
			<AlertTriangle size={16} />
			<span>LaTeX Compilation Errors ({errorCount})</span>
		</button>

		<div class="flex items-center gap-2">
			<Button
				variant="outline"
				size="sm"
				onclick={copyErrors}
				class="h-6 gap-1.5 px-2 text-red-700 border-red-300 hover:bg-red-100"
			>
				<Copy size={12} />
				{copySuccess ? 'Copied!' : 'Copy'}
			</Button>
			
			<button
				onclick={toggleExpanded}
				class="text-red-600 hover:text-red-800"
				title={isExpanded ? 'Hide Errors' : 'Show Errors'}
			>
				{#if isExpanded}
					<ChevronDown size={16} />
				{:else}
					<ChevronRight size={16} />
				{/if}
			</button>
		</div>
	</div>

	<!-- Error Content -->
	{#if isExpanded}
	<div class="max-h-64 overflow-auto p-3 bg-red-50">
		<pre class="text-xs font-mono text-red-800 whitespace-pre-wrap leading-relaxed">{errorText}</pre>
	</div>
	{/if}
</div>
{/if}

<style>
	/* Custom scrollbar for error content */
	.overflow-auto {
		scrollbar-width: thin;
		scrollbar-color: rgba(239, 68, 68, 0.3) rgba(239, 68, 68, 0.1);
	}
	
	.overflow-auto::-webkit-scrollbar {
		width: 8px;
	}
	
	.overflow-auto::-webkit-scrollbar-track {
		background: rgba(239, 68, 68, 0.1);
	}
	
	.overflow-auto::-webkit-scrollbar-thumb {
		background: rgba(239, 68, 68, 0.3);
		border-radius: 4px;
	}
	
	.overflow-auto::-webkit-scrollbar-thumb:hover {
		background: rgba(239, 68, 68, 0.5);
	}
</style>