<script lang="ts">
	import { fly, fade } from 'svelte/transition';
	import { Loader2 } from '@lucide/svelte';
	
	interface Props {
		text: string;
		visible?: boolean;
	}
	
	let { text, visible = false }: Props = $props();
</script>

{#if visible}
	<div
		transition:fly={{ y: 10, duration: 150 }}
		class="inline-block"
	>
		<div
			class="bg-background/90 backdrop-blur-sm rounded-full shadow-md border px-3 py-1.5"
			transition:fade={{ duration: 100 }}
		>
			<div class="flex items-center gap-1.5">
				<Loader2 size={12} class="animate-spin text-muted-foreground" />
				<span class="text-xs text-muted-foreground">{text}</span>
			</div>
		</div>
	</div>
{/if}