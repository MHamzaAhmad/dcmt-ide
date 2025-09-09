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
	<div class="space-y-1 mb-4">
		{#each sortedResults as result (result.tool_call_id)}
			{#if result.status === 'executing'}
				<div class="text-xs text-muted-foreground animate-pulse">
					Using {formatToolName(result.tool_name)}...
				</div>
			{/if}
		{/each}
	</div>
{/if}