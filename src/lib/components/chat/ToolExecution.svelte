<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';
	import { Card, CardContent, CardHeader } from '$lib/components/ui/card';
	import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '$lib/components/ui/collapsible';
	import { Button } from '$lib/components/ui/button';
	import type { AgentToolResult } from '$lib/api/types';
	import { 
		Settings, 
		CheckCircle, 
		XCircle, 
		Clock, 
		ChevronDown, 
		ChevronRight,
		FileText,
		Edit,
		FolderPlus,
		List,
		Trash2,
		Play
	} from '@lucide/svelte';
	import { formatDistanceToNow } from 'date-fns';

	interface Props {
		toolResults: Map<string, AgentToolResult>;
		showAll?: boolean;
	}
	
	let { toolResults, showAll = false }: Props = $props();

	let expandedResults = $state(new Set<string>());

	// Convert Map to array for iteration
	const resultsArray = $derived(Array.from(toolResults.values()).filter(result => {
		// Show executing tools and recently completed tools
		return result.status === 'executing' || 
			   (result.status === 'completed' && showAll) ||
			   (result.status === 'error');
	}));

	// Sort by started_at time, most recent first
	const sortedResults = $derived(resultsArray.sort((a, b) => {
		const aTime = a.started_at?.getTime() || 0;
		const bTime = b.started_at?.getTime() || 0;
		return bTime - aTime;
	}));

	function toggleExpanded(toolCallId: string) {
		const newSet = new Set(expandedResults);
		if (newSet.has(toolCallId)) {
			newSet.delete(toolCallId);
		} else {
			newSet.add(toolCallId);
		}
		expandedResults = newSet;
	}

	function getToolIcon(toolName: string) {
		switch (toolName) {
			case 'read_file':
				return FileText;
			case 'write_file':
			case 'update_file':
				return Edit;
			case 'create_directory':
				return FolderPlus;
			case 'list_files':
				return List;
			case 'delete_file':
				return Trash2;
			default:
				return Settings;
		}
	}

	function getStatusIcon(status: AgentToolResult['status']) {
		switch (status) {
			case 'executing':
				return Clock;
			case 'completed':
				return CheckCircle;
			case 'error':
				return XCircle;
			default:
				return Play;
		}
	}

	function getStatusBadgeVariant(status: AgentToolResult['status']) {
		switch (status) {
			case 'executing':
				return 'secondary' as const;
			case 'completed':
				return 'default' as const;
			case 'error':
				return 'destructive' as const;
			default:
				return 'outline' as const;
		}
	}

	function formatToolName(toolName: string): string {
		return toolName
			.split('_')
			.map(word => word.charAt(0).toUpperCase() + word.slice(1))
			.join(' ');
	}

	function formatDuration(started: Date, ended?: Date): string {
		const endTime = ended || new Date();
		const duration = endTime.getTime() - started.getTime();
		
		if (duration < 1000) {
			return `${duration}ms`;
		} else if (duration < 60000) {
			return `${(duration / 1000).toFixed(1)}s`;
		} else {
			return `${(duration / 60000).toFixed(1)}m`;
		}
	}

	function truncateResult(result: string, maxLength = 200): string {
		if (result.length <= maxLength) return result;
		return result.substring(0, maxLength) + '...';
	}
</script>

{#if sortedResults.length > 0}
	<div class="space-y-2 mb-4">
		<div class="flex items-center gap-2 text-sm text-muted-foreground">
			<Settings size={14} />
			<span>Tool Execution</span>
			{#if sortedResults.length > 1}
				<Badge variant="outline" class="text-xs">
					{sortedResults.length} tools
				</Badge>
			{/if}
			{#if sortedResults.some(r => r.status === 'executing')}
				<Badge variant="default" class="text-xs gap-1 animate-pulse">
					<div class="w-2 h-2 bg-current rounded-full animate-ping"></div>
					Active
				</Badge>
			{/if}
		</div>

		<div class="space-y-2">
			{#each sortedResults as result, index (result.tool_call_id)}
				{@const ToolIcon = getToolIcon(result.tool_name)}
				{@const StatusIcon = getStatusIcon(result.status)}
				{@const isExpanded = expandedResults.has(result.tool_call_id)}
				{@const hasResult = result.result && result.result.trim().length > 0}
				{@const hasError = result.error && result.error.trim().length > 0}

				<Card class="border-l-4 {result.status === 'executing' ? 'border-l-blue-500 bg-blue-50/30 dark:bg-blue-950/10' : result.status === 'completed' ? 'border-l-green-500' : 'border-l-red-500'} transition-all duration-300">
					<CardHeader class="pb-2">
						<Collapsible>
							<CollapsibleTrigger 
								class="flex items-center justify-between w-full text-left hover:bg-muted/50 rounded p-2 -m-2 transition-colors"
								onclick={() => toggleExpanded(result.tool_call_id)}
							>
								<div class="flex items-center gap-3">
									<div class="flex items-center gap-2">
										<!-- Sequence indicator -->
										<div class="w-6 h-6 rounded-full bg-muted text-muted-foreground flex items-center justify-center text-xs font-mono {result.status === 'executing' ? 'bg-blue-500 text-white animate-pulse' : result.status === 'completed' ? 'bg-green-500 text-white' : result.status === 'error' ? 'bg-red-500 text-white' : ''}">
											{sortedResults.length - index}
										</div>
										
										<div class="relative">
											<ToolIcon size={16} class="text-muted-foreground {result.status === 'executing' ? 'animate-pulse' : ''}" />
											{#if result.status === 'executing'}
												<div class="absolute -top-1 -right-1 w-3 h-3">
													<div class="w-full h-full bg-blue-500 rounded-full animate-ping opacity-75"></div>
													<div class="absolute top-0 left-0 w-full h-full bg-blue-500 rounded-full"></div>
												</div>
											{/if}
										</div>
										<span class="font-medium text-sm">
											{formatToolName(result.tool_name)}
										</span>
									</div>
									
									<Badge variant={getStatusBadgeVariant(result.status)} class="gap-1 text-xs {result.status === 'executing' ? 'animate-pulse' : ''}">
										<StatusIcon size={12} class={result.status === 'executing' ? 'animate-spin' : ''} />
										{result.status}
										{#if result.status === 'executing'}
											<div class="flex gap-0.5">
												<div class="w-1 h-1 bg-current rounded-full animate-bounce" style="animation-delay: 0ms"></div>
												<div class="w-1 h-1 bg-current rounded-full animate-bounce" style="animation-delay: 100ms"></div>
												<div class="w-1 h-1 bg-current rounded-full animate-bounce" style="animation-delay: 200ms"></div>
											</div>
										{/if}
									</Badge>
								</div>

								<div class="flex items-center gap-2 text-xs text-muted-foreground">
									{#if result.started_at}
										{#if result.status === 'executing'}
											<span>{formatDistanceToNow(result.started_at, { addSuffix: true })}</span>
										{:else if result.completed_at}
											<span>{formatDuration(result.started_at, result.completed_at)}</span>
										{/if}
									{/if}
									
									<Button variant="ghost" size="icon" class="h-4 w-4">
										{#if isExpanded}
											<ChevronDown size={12} />
										{:else}
											<ChevronRight size={12} />
										{/if}
									</Button>
								</div>
							</CollapsibleTrigger>

							<CollapsibleContent>
								<CardContent class="pt-2">
									{#if hasError}
										<div class="bg-destructive/10 border border-destructive/20 rounded p-3 mb-3">
											<div class="flex items-center gap-2 mb-2">
												<XCircle size={14} class="text-destructive" />
												<span class="text-sm font-medium text-destructive">Error</span>
											</div>
											<pre class="text-xs text-destructive whitespace-pre-wrap font-mono overflow-x-auto">{result.error}</pre>
										</div>
									{/if}

									{#if hasResult}
										<div class="bg-muted/50 rounded p-3">
											<div class="flex items-center justify-between mb-2">
												<div class="flex items-center gap-2">
													<CheckCircle size={14} class="text-green-600" />
													<span class="text-sm font-medium">Result</span>
												</div>
												
												{#if result.result && result.result.length > 200}
													<Button 
														variant="ghost" 
														size="sm" 
														class="h-6 text-xs"
														onclick={() => navigator.clipboard.writeText(result.result || '')}
													>
														Copy
													</Button>
												{/if}
											</div>
											
											<div class="text-xs">
												{#if result.result && result.result.length > 200}
													{#if isExpanded}
														<pre class="whitespace-pre-wrap font-mono overflow-x-auto">{result.result}</pre>
													{:else}
														<p class="font-mono">{truncateResult(result.result)}</p>
														<Button 
															variant="ghost" 
															size="sm" 
															class="h-6 text-xs mt-2 p-0"
															onclick={() => toggleExpanded(result.tool_call_id)}
														>
															Show more
														</Button>
													{/if}
												{:else}
													<pre class="whitespace-pre-wrap font-mono overflow-x-auto">{result.result}</pre>
												{/if}
											</div>
										</div>
									{:else if result.status === 'executing'}
										<div class="bg-gradient-to-r from-blue-50 to-blue-100 dark:from-blue-950/20 dark:to-blue-900/20 border border-blue-200 dark:border-blue-800 rounded p-3 animate-pulse">
											<div class="flex items-center gap-3">
												<div class="relative">
													<div class="w-4 h-4 border-2 border-blue-300 border-t-blue-600 rounded-full animate-spin"></div>
												</div>
												<div class="flex flex-col">
													<span class="text-sm font-medium text-blue-700 dark:text-blue-300">Executing {formatToolName(result.tool_name)}...</span>
													<div class="flex items-center gap-1 mt-1">
														<div class="h-1 w-16 bg-blue-200 dark:bg-blue-800 rounded-full overflow-hidden">
															<div class="h-full bg-blue-500 animate-pulse" style="width: {Math.random() * 80 + 20}%"></div>
														</div>
														<span class="text-xs text-blue-600 dark:text-blue-400">Working...</span>
													</div>
												</div>
											</div>
										</div>
									{/if}

									{#if result.status === 'completed' && result.completed_at && result.started_at}
										<div class="flex items-center gap-4 mt-3 pt-2 border-t text-xs text-muted-foreground">
											<div class="flex items-center gap-1">
												<Clock size={10} />
												<span>Duration: {formatDuration(result.started_at, result.completed_at)}</span>
											</div>
											<div>
												<span>Completed {formatDistanceToNow(result.completed_at, { addSuffix: true })}</span>
											</div>
										</div>
									{/if}
								</CardContent>
							</CollapsibleContent>
						</Collapsible>
					</CardHeader>
				</Card>
			{/each}
		</div>
	</div>
{/if}