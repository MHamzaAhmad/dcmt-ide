<script lang="ts">
	import { fly, fade } from 'svelte/transition';
	import { Badge } from '$lib/components/ui/badge';
	import { Loader2 } from '@lucide/svelte';
	
	interface Props {
		text: string;
		visible?: boolean;
		variant?: 'default' | 'secondary' | 'destructive' | 'outline';
	}
	
	let { text, visible = false, variant = 'secondary' }: Props = $props();
</script>

{#if visible}
	<div
		class="fixed bottom-8 left-1/2 -translate-x-1/2 z-50"
		transition:fly={{ y: 20, duration: 200 }}
	>
		<div
			class="bg-background/95 backdrop-blur-sm rounded-full shadow-lg border px-4 py-2"
			transition:fade={{ duration: 150 }}
		>
			<div class="flex items-center gap-2">
				<Loader2 size={14} class="animate-spin text-primary" />
				<span class="text-sm font-medium">{text}</span>
			</div>
		</div>
	</div>
{/if}