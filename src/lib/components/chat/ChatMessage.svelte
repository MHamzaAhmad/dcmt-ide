<script lang="ts">
	import type { ChatMessage, AgentChatMessage, AgentToolCall } from '$lib/api/types';
	import { Badge } from '$lib/components/ui/badge';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { User, Bot, AlertCircle, CheckCircle, Clock, Settings, Loader2 } from '@lucide/svelte';
	import { formatDistanceToNow } from 'date-fns';
	
	interface Props {
		message: ChatMessage | AgentChatMessage;
		showTimestamp?: boolean;
	}
	
	let { message, showTimestamp = true }: Props = $props();
	
	// Type guard to check if message is an AgentChatMessage
	function isAgentMessage(msg: ChatMessage | AgentChatMessage): msg is AgentChatMessage {
		return 'status' in msg || 'tool_calls' in msg || 'streaming' in msg;
	}
	
	// Get status icon and color
	function getStatusInfo(status: AgentChatMessage['status']) {
		switch (status) {
			case 'sending':
				return { icon: Loader2, class: 'text-blue-500 animate-spin', text: 'Sending...' };
			case 'streaming':
				return { icon: Clock, class: 'text-blue-500 animate-pulse', text: 'Thinking...' };
			case 'tool_execution':
				return { icon: Settings, class: 'text-yellow-500', text: 'Using tools...' };
			case 'completed':
				return { icon: CheckCircle, class: 'text-green-500', text: 'Completed' };
			case 'error':
				return { icon: AlertCircle, class: 'text-red-500', text: 'Error' };
			default:
				return null;
		}
	}
	
	// Format tool call arguments for display
	function formatToolArguments(args: string): Record<string, any> {
		try {
			return JSON.parse(args);
		} catch {
			return { arguments: args };
		}
	}
	
	// Format tool name for display
	function formatToolName(name: string): string {
		return name.split('_').map(word => 
			word.charAt(0).toUpperCase() + word.slice(1)
		).join(' ');
	}
	
	const agentMessage = $derived(isAgentMessage(message) ? message : null);
	const statusInfo = $derived(agentMessage?.status ? getStatusInfo(agentMessage.status) : null);
</script>

<div class="py-1 px-4 {message.role === 'user' ? 'text-slate-500 dark:text-slate-400' : 'text-slate-900 dark:text-slate-100'} mb-2">
	
	<div class="flex-1">
		
		<!-- Message Content -->
		{#if message.content}
			<div class="text-sm whitespace-pre-wrap leading-relaxed">
				{message.content}
			</div>
		{/if}
		
		<!-- Error Display - Minimal -->
		{#if agentMessage?.error}
			<div class="text-sm text-red-500 opacity-75">
				Error: {agentMessage.error}
			</div>
		{/if}
		
	</div>
</div>