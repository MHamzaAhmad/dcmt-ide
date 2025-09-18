<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Separator } from '$lib/components/ui/separator';
	import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '$lib/components/ui/tooltip';
	import { 
		FolderOpen, 
		GitBranch, 
		Sun, 
		Moon 
	} from '@lucide/svelte';
	import { theme } from '$lib/stores/theme.js';
	import { editorState } from '$lib/stores/editor.js';
	import { billingStore } from '$lib/stores/billing';

	let currentTheme = $state('light');
	let isFileExplorerOpen = $state(true);
	let isVersionControlOpen = $state(false);
	const billing = $derived($billingStore);
	let upgrading = $state(false);
	async function handleUpgrade() { upgrading = true; await billingStore.upgrade(); upgrading = false; }

	$effect(() => {
		currentTheme = $theme;
	});

	$effect(() => {
		isFileExplorerOpen = $editorState.isFileExplorerOpen;
		isVersionControlOpen = $editorState.isVersionControlOpen;
	});
</script>

<TooltipProvider>
	<header class="h-12 border-b bg-background flex items-center px-3 gap-2">
		<!-- Logo/Brand -->
		<div class="flex items-center gap-2 mr-4">
			<span class="text-sm font-medium">Researgent Editor</span>
		</div>

		<Separator orientation="vertical" />

		<!-- File Explorer Toggle -->
		<Tooltip>
			<TooltipTrigger>
				<Button
					variant={isFileExplorerOpen ? 'default' : 'ghost'}
					size="sm"
					onclick={() => editorState.toggleFileExplorer()}
				>
					<FolderOpen size={16} />
				</Button>
			</TooltipTrigger>
			<TooltipContent>
				<p>Toggle File Explorer</p>
			</TooltipContent>
		</Tooltip>

		<!-- Version Control Toggle -->
		<Tooltip>
			<TooltipTrigger>
				<Button
					variant={isVersionControlOpen ? 'default' : 'ghost'}
					size="sm"
					onclick={() => editorState.toggleVersionControl()}
				>
					<GitBranch size={16} />
				</Button>
			</TooltipTrigger>
			<TooltipContent>
				<p>Toggle Version Control</p>
			</TooltipContent>
		</Tooltip>

		<!-- Spacer -->
		<div class="flex-1"></div>

		<!-- Theme Toggle -->
		{#if billing.isReady && !billing.hasActiveSubscription}
			<Button variant="default" size="sm" onclick={handleUpgrade} disabled={upgrading}>
				{upgrading ? 'Redirecting…' : 'Upgrade'}
			</Button>
		{/if}
		<Tooltip>
			<TooltipTrigger>
				<Button
					variant="ghost"
					size="sm"
					onclick={() => theme.toggle()}
				>
					{#if currentTheme === 'light'}
						<Sun size={16} />
					{:else}
						<Moon size={16} />
					{/if}
				</Button>
			</TooltipTrigger>
			<TooltipContent>
				<p>Toggle Theme</p>
			</TooltipContent>
		</Tooltip>
	</header>
</TooltipProvider>