<script lang="ts">
	import type { ChatMessage } from '$lib/api/types';
	import { User, Bot } from '@lucide/svelte';
	
	interface Props {
		message: ChatMessage;
	}
	
	let { message }: Props = $props();
</script>

<div class="flex gap-3 p-4 {message.role === 'user' ? 'bg-background' : 'bg-muted/30'}">
	<div class="flex-shrink-0">
		{#if message.role === 'user'}
			<div class="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center">
				<User size={16} class="text-primary" />
			</div>
		{:else}
			<div class="w-8 h-8 rounded-full bg-muted flex items-center justify-center">
				<Bot size={16} class="text-muted-foreground" />
			</div>
		{/if}
	</div>
	
	<div class="flex-1 space-y-1">
		<div class="flex items-center gap-2">
			<span class="text-sm font-medium">
				{message.role === 'user' ? 'You' : 'Assistant'}
			</span>
			{#if message.model}
				<span class="text-xs text-muted-foreground">• {message.model}</span>
			{/if}
		</div>
		<div class="text-sm whitespace-pre-wrap">
			{message.content}
		</div>
	</div>
</div>