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

<div class="flex gap-3 p-4 {message.role === 'user' ? 'bg-background' : 'bg-muted/30'}">
	<div class="flex-shrink-0">
		{#if message.role === 'user'}
			<div class="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center">
				<User size={16} class="text-primary" />
			</div>
		{:else if message.role === 'system'}
			<div class="w-8 h-8 rounded-full bg-muted flex items-center justify-center">
				<Settings size={16} class="text-muted-foreground" />
			</div>
		{:else}
			<div class="w-8 h-8 rounded-full bg-muted flex items-center justify-center">
				<Bot size={16} class="text-muted-foreground" />
			</div>
		{/if}
	</div>
	
	<div class="flex-1 space-y-2">
		<!-- Message Header -->
		<div class="flex items-center gap-2 flex-wrap">
			<span class="text-sm font-medium">
				{message.role === 'user' ? 'You' : message.role === 'system' ? 'System' : 'Assistant'}
			</span>
			
			{#if message.model}
				<span class="text-xs text-muted-foreground">• {message.model}</span>
			{/if}
			
			{#if statusInfo}
				{@const StatusIcon = statusInfo.icon}
				<Badge variant="outline" class="gap-1 text-xs">
					<StatusIcon size={12} class={statusInfo.class} />
					{statusInfo.text}
				</Badge>
			{/if}
			
			{#if showTimestamp && message.timestamp}
				<span class="text-xs text-muted-foreground">
					{formatDistanceToNow(message.timestamp, { addSuffix: true })}
				</span>
			{/if}
			
			{#if agentMessage?.job_id}
				<span class="text-xs text-muted-foreground font-mono">
					#{agentMessage.job_id.slice(-8)}
				</span>
			{/if}
		</div>
		
		<!-- Tool Calls Display (for messages with tool calls) -->
		{#if agentMessage?.tool_calls && agentMessage.tool_calls.length > 0}
			<div class="space-y-2">
				{#each agentMessage.tool_calls as toolCall}
					{@const args = formatToolArguments(toolCall.function.arguments)}
					<Card class="border-l-4 border-l-blue-500">
						<CardContent class="p-3">
							<div class="flex items-center gap-2 mb-2">
								<Settings size={14} class="text-muted-foreground" />
								<span class="text-sm font-medium">
									{formatToolName(toolCall.function.name)}
								</span>
								<Badge variant="secondary" class="text-xs">Tool Call</Badge>
							</div>
							
							{#if Object.keys(args).length > 0}
								<div class="bg-muted/50 rounded p-2 text-xs">
									<div class="text-muted-foreground mb-1">Arguments:</div>
									<pre class="whitespace-pre-wrap font-mono overflow-x-auto">{JSON.stringify(args, null, 2)}</pre>
								</div>
							{/if}
						</CardContent>
					</Card>
				{/each}
			</div>
		{/if}
		
		<!-- Message Content -->
		{#if message.content}
			<div class="text-sm whitespace-pre-wrap">
				{message.content}
				{#if agentMessage?.streaming && agentMessage?.status === 'streaming'}
					<span class="animate-pulse">▋</span>
				{/if}
			</div>
		{:else if agentMessage?.streaming && agentMessage?.status === 'streaming'}
			<div class="text-sm text-muted-foreground italic">
				<span class="animate-pulse">Thinking...</span>
			</div>
		{/if}
		
		<!-- Error Display -->
		{#if agentMessage?.error}
			<Card class="border-l-4 border-l-red-500">
				<CardContent class="p-3">
					<div class="flex items-center gap-2 mb-2">
						<AlertCircle size={14} class="text-red-500" />
						<span class="text-sm font-medium text-red-700 dark:text-red-300">Error</span>
					</div>
					<div class="text-sm text-red-600 dark:text-red-400">
						{agentMessage.error}
					</div>
				</CardContent>
			</Card>
		{/if}
		
	</div>
</div>