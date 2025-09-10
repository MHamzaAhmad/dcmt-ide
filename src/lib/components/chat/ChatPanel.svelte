<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import * as Select from '$lib/components/ui/select';
	import { Badge } from '$lib/components/ui/badge';
	import ChatMessage from './ChatMessage.svelte';
	import ToolExecution from './ToolExecution.svelte';
	import FloatingBadge from '../ui/floating-badge.svelte';
	import { chatStore } from '$lib/stores/chat';
	import { agentStore } from '$lib/stores/agent';
	import { editorState } from '$lib/stores/editor';
	import { modelsAPI } from '$lib/api/models';
	import type { ChatMessage as ChatMessageType, AgentChatMessage, LiteLLMModel } from '$lib/api/types';
	import { Send, Bot, AlertCircle, Loader2, Maximize2, Minimize2, ChevronDown, ChevronUp, ArrowDown } from '@lucide/svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	
	interface Props {
		mode?: 'docked' | 'floating';
	}
	
	let { mode = 'docked' }: Props = $props();
	
	let inputValue = $state('');
	let scrollContainer = $state<HTMLDivElement>();
	let messages = $state<AgentChatMessage[]>([]);
	let selectedModel = $state<LiteLLMModel | null>(null);
	let selectedModelId = $state('');
	let isLoading = $state(false);
	let isAgentMode = $state(true); // Toggle between agent and simple chat
	let isExpanded = $state(false); // For floating mode expand/collapse
	let isMinimized = $state(false); // For floating mode minimize
	let showScrollToBottom = $state(false);
	let autoScroll = $state(true);
	
	// Agent state
	let agentAvailable = $state(false);
	let agentError = $state<string | null>(null);
	let toolResults = $state(new Map());
	let streamingContent = $state('');
	let isProcessing = $state(false);
	let currentToolStatus = $state<{ toolName: string; status: string } | null>(null);
	
	// Query for fetching models from LiteLLM
	const modelsQuery = createQuery({
		queryKey: ['litellm-models'],
		queryFn: () => modelsAPI.listLiteLLMModels(),
		staleTime: 5 * 60 * 1000, // 5 minutes
		retry: 2,
		enabled: () => isAgentMode
	});
	
	// Subscribe to agent store
	$effect(() => {
		if (isAgentMode) {
			const agentState = $agentStore;
			agentAvailable = agentState.isAvailable;
			agentError = agentState.connectionError;
			messages = agentState.messages;
			selectedModel = agentState.selectedModel;
			toolResults = agentState.activeToolResults;
			streamingContent = agentState.streamingContent;
			isProcessing = agentState.isProcessing;
			currentToolStatus = agentState.currentToolStatus;
		} else {
			// Use legacy chat store for simple chat
			messages = $chatStore.messages as AgentChatMessage[];
			selectedModel = $chatStore.selectedModel ? {
				id: $chatStore.selectedModel.id,
				object: 'model',
				created: Date.now() / 1000,
				owned_by: 'unknown'
			} : null;
			isLoading = $chatStore.isLoading;
		}
	});
	
	// Derive selectedModelId from selectedModel (one-way binding)
	$effect(() => {
		if (selectedModel && selectedModel.id !== selectedModelId) {
			selectedModelId = selectedModel.id;
		}
	});
	
	// Update available models when query succeeds
	$effect(() => {
		if (isAgentMode && $modelsQuery.data?.data) {
			// Update agent store with LiteLLM models
			agentStore.update(state => ({
				...state,
				availableModels: $modelsQuery.data.data,
				selectedModel: state.selectedModel || $modelsQuery.data.data[0] || null
			}));
		}
	});
	
	// Handle model selection from dropdown
	function handleModelSelection(modelId: string) {
		if (!modelId) return;
		
		if (isAgentMode) {
			const model = $agentStore.availableModels.find(m => m.id === modelId);
			if (model) {
				agentStore.setSelectedModel(model);
			}
		} else {
			const model = $chatStore.availableModels.find(m => m.id === modelId);
			if (model) {
				chatStore.setSelectedModel(model);
			}
		}
	}
	
	// Derive trigger content
	const triggerContent = $derived(
		isAgentMode ? (
			$modelsQuery.isLoading ? "Loading models..." :
			!agentAvailable ? "Agent unavailable" :
			selectedModel ? selectedModel.id : "Select a model"
		) : (
			$modelsQuery.isLoading ? "Loading models..." :
			selectedModel ? selectedModel.id : "Select a model"
		)
	);
	
	// Auto-scroll to bottom when new messages arrive (if auto-scroll enabled)
	$effect(() => {
		if (scrollContainer && autoScroll && (messages.length > 0 || toolResults.size > 0 || streamingContent)) {
			requestAnimationFrame(() => {
				if (scrollContainer && autoScroll) {
					scrollContainer.scrollTop = scrollContainer.scrollHeight;
				}
			});
		}
	});
	
	// Monitor scroll position to show/hide scroll-to-bottom button
	function handleScroll() {
		if (!scrollContainer) return;
		
		const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
		const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
		
		// Show button if user scrolled up more than 100px from bottom
		showScrollToBottom = distanceFromBottom > 100;
		
		// Disable auto-scroll if user manually scrolled up
		if (distanceFromBottom > 50) {
			autoScroll = false;
		} else {
			autoScroll = true;
		}
	}
	
	function scrollToBottom() {
		if (scrollContainer) {
			scrollContainer.scrollTo({
				top: scrollContainer.scrollHeight,
				behavior: 'smooth'
			});
			autoScroll = true;
		}
	}
	
	// Get queryClient in component context
	const queryClient = useQueryClient();
	
	onMount(async () => {
		// Pass queryClient to agent store
		agentStore.setQueryClient(queryClient);
		
		// Initialize agent system
		if (isAgentMode) {
			await agentStore.initialize();
			
			// Create initial session if we don't have one
			if (!$agentStore.currentSessionId) {
				await agentStore.createSession();
			}
		}
	});
	
	onDestroy(() => {
		if (isAgentMode) {
			agentStore.destroy();
		}
	});
	
	async function handleSend() {
		if (!inputValue.trim() || !selectedModel) return;
		
		if (isAgentMode && agentAvailable) {
			// Use agent system
			try {
				await agentStore.sendMessage(inputValue.trim());
				inputValue = '';
			} catch (error) {
				console.error('Failed to send message:', error);
				// TODO: Show error notification
			}
		} else {
			// Legacy simple chat
			const newMessage: AgentChatMessage = {
				id: Date.now().toString(),
				role: 'user',
				content: inputValue.trim(),
				timestamp: new Date(),
				status: 'completed'
			};
			
			chatStore.addMessage(newMessage as ChatMessageType);
			inputValue = '';
			
			// In simple chat mode, messages are stored but no response is generated
			// The user should enable agent mode for AI assistance
		}
	}
	
	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			handleSend();
		}
	}
	
	async function handleClearSession() {
		if (isAgentMode) {
			await agentStore.clearSession();
			await agentStore.createSession();
		} else {
			chatStore.clearMessages();
		}
	}
	
	
	function handleFullscreen() {
		// Switch to chat tab when maximizing from floating mode
		if (mode === 'floating') {
			editorState.setActiveTab('chat');
		}
	}
	
	function toggleExpanded() {
		isExpanded = !isExpanded;
	}
	
	function toggleMinimized() {
		isMinimized = !isMinimized;
	}
	
	function getToolStatusText(toolName: string): string {
		const toolStatusMap: Record<string, string> = {
			'read_file': 'Reading file',
			'write_file': 'Writing file',
			'update_file': 'Updating file',
			'create_file': 'Creating file',
			'create_directory': 'Creating directory',
			'list_files': 'Listing files',
			'delete_file': 'Deleting file',
			'search_files': 'Searching files',
			'execute_command': 'Executing command'
		};
		
		return toolStatusMap[toolName] || 'Processing';
	}
</script>

<div class="{mode === 'floating' ? 
	'absolute bottom-4 left-4 right-4 bg-background/95 backdrop-blur border rounded-lg shadow-lg transition-all duration-300 z-50' + 
	(isMinimized ? ' h-12' : isExpanded ? ' h-[600px]' : ' h-80') : 
	'h-full bg-background'} flex flex-col">
	<!-- Header with Mode Toggle -->
	<div class="{mode === 'floating' ? 'border-b p-2 bg-muted/30' : 'border-b p-3'}">
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-2">
				{#if mode === 'floating'}
					<Bot size={16} class="text-primary" />
					<span class="text-sm font-medium">AI Assistant</span>
				{:else}
					<Bot size={20} class="text-primary" />
					<h2 class="text-lg font-semibold">AI Assistant</h2>
				{/if}
			</div>
			
			<div class="flex items-center gap-1">
				{#if mode === 'floating'}
					{#if !isMinimized}
						<Button
							onclick={toggleExpanded}
							size="icon"
							variant="ghost"
							class="h-7 w-7"
							title={isExpanded ? 'Collapse' : 'Expand'}
						>
							{#if isExpanded}
								<ChevronDown size={14} />
							{:else}
								<ChevronUp size={14} />
							{/if}
						</Button>
					{/if}
					<Button
						onclick={toggleMinimized}
						size="icon"
						variant="ghost"
						class="h-7 w-7"
						title={isMinimized ? 'Restore' : 'Minimize'}
					>
						<Minimize2 size={14} />
					</Button>
					<Button
						onclick={handleFullscreen}
						size="icon"
						variant="ghost"
						class="h-7 w-7"
						title="Open in fullscreen"
					>
						<Maximize2 size={14} />
					</Button>
				{:else}
					<Button variant="ghost" size="sm" onclick={handleClearSession}>
						Clear
					</Button>
				{/if}
			</div>
		</div>
		
		{#if isAgentMode && agentError}
			<div class="mt-2 text-sm text-destructive">
				{agentError}
			</div>
		{/if}
	</div>

	<!-- Chat Messages Area -->
	{#if !isMinimized}
	<div class="flex-1 overflow-hidden relative">
		<div 
			bind:this={scrollContainer} 
			class="h-full overflow-y-auto"
			onscroll={handleScroll}
		>
			{#if messages.length === 0}
				<div class="flex flex-col items-center justify-center h-full p-8 text-center">
					<Bot size={48} class="text-muted-foreground mb-4" />
					<h3 class="text-lg font-medium mb-2">Start a conversation</h3>
					<p class="text-sm text-muted-foreground max-w-md">
						{#if isAgentMode}
							Ask questions about your LaTeX document or request help with file operations. The agent can read, write, and modify files in your workspace.
						{:else}
							Simple chat mode for basic conversations. Enable agent mode for file operations and advanced features.
						{/if}
					</p>
				</div>
			{:else}
				<div class="pb-4">
					{#each messages as message}
						<ChatMessage {message} />
					{/each}
					
					<!-- Tool Execution Display -->
					{#if isAgentMode && toolResults.size > 0}
						<div class="px-4">
							<ToolExecution {toolResults} />
						</div>
					{/if}
				</div>
			{/if}
		</div>
		
		<!-- Floating Badge for Tool Status - Now inside chat panel -->
		{#if currentToolStatus && isAgentMode}
			<div class="absolute bottom-4 left-1/2 -translate-x-1/2 z-20 pointer-events-none">
				<FloatingBadge 
					text={getToolStatusText(currentToolStatus.toolName)}
					visible={true}
				/>
			</div>
		{/if}
		
		<!-- Scroll to Bottom Button -->
		{#if showScrollToBottom}
			<button
				onclick={scrollToBottom}
				class="absolute bottom-4 right-4 p-2 bg-primary text-primary-foreground rounded-full shadow-lg hover:shadow-xl transition-all duration-200 hover:scale-110 z-10"
				title="Scroll to bottom"
			>
				<ArrowDown size={20} />
			</button>
		{/if}
	</div>
	
	<!-- Input Area -->
	<div class="{mode === 'floating' ? 'border-t p-3' : 'border-t p-4'}">
		<div class="{mode === 'floating' ? 'flex gap-2' : 'flex gap-2 mb-3'}">
			<Select.Root 
				type="single" 
				value={selectedModelId}
				onValueChange={handleModelSelection}
				disabled={$modelsQuery.isLoading}
			>
				<Select.Trigger class={mode === 'floating' ? 'w-[140px] h-8' : 'w-[200px] h-9'}>
					{triggerContent}
				</Select.Trigger>
				<Select.Content>
					{#if $modelsQuery.isError}
						<div class="flex items-center gap-2 p-2 text-sm text-destructive">
							<AlertCircle size={14} />
							<span>Failed to load models</span>
						</div>
					{:else if isAgentMode && $agentStore.availableModels.length > 0}
						<Select.Group>
							<Select.Label>Models</Select.Label>
							{#each $agentStore.availableModels as model (model.id)}
								<Select.Item value={model.id} label={model.id}>
									<div class="flex items-center justify-between w-full">
										<span>{model.id}</span>
										<Badge variant="outline" class="text-xs ml-2">
											{model.owned_by}
										</Badge>
									</div>
								</Select.Item>
							{/each}
						</Select.Group>
					{:else if !isAgentMode && $chatStore.availableModels.length > 0}
						<Select.Group>
							<Select.Label>Available Models</Select.Label>
							{#each $chatStore.availableModels as model (model.id)}
								<Select.Item value={model.id} label={model.name}>
									{model.name}
								</Select.Item>
							{/each}
						</Select.Group>
					{:else}
						<div class="p-2 text-sm text-muted-foreground">
							{isAgentMode ? 'No LiteLLM models available' : 'No models available'}
						</div>
					{/if}
				</Select.Content>
			</Select.Root>
			
			{#if mode === 'floating'}
				<Input
					bind:value={inputValue}
					placeholder={selectedModel ? 'Ask about your LaTeX project...' : 'Select a model first...'}
					disabled={!selectedModel || (isAgentMode ? isProcessing : isLoading)}
					onkeydown={handleKeyDown}
					class="flex-1 h-8"
				/>
				<Button 
					onclick={handleSend}
					disabled={!inputValue.trim() || !selectedModel || (isAgentMode ? isProcessing : isLoading)}
					size="icon"
					class="h-8 w-8"
				>
					{#if isAgentMode ? isProcessing : isLoading}
						<Loader2 size={14} class="animate-spin" />
					{:else}
						<Send size={14} />
					{/if}
				</Button>
			{/if}
		</div>
		
		{#if mode === 'docked'}
		<div class="flex gap-2 relative">
			{#if (isAgentMode ? isProcessing : isLoading)}
				<div class="absolute inset-0 bg-gradient-to-r from-blue-500/10 via-purple-500/10 to-blue-500/10 animate-pulse rounded-lg -z-10" style="animation-duration: 2s;"></div>
			{/if}
			<Input
				bind:value={inputValue}
				placeholder={selectedModel ? ((isAgentMode ? isProcessing : isLoading) ? "AI is working..." : "Ask me to help with your LaTeX project...") : "Select a model first..."}
				disabled={!selectedModel || (isAgentMode ? isProcessing : isLoading)}
				onkeydown={handleKeyDown}
				class="flex-1 {(isAgentMode ? isProcessing : isLoading) ? 'opacity-75' : ''}"
			/>
			<Button 
				onclick={handleSend}
				disabled={!inputValue.trim() || !selectedModel || (isAgentMode ? isProcessing : isLoading)}
				size="icon"
			>
				{#if isAgentMode ? isProcessing : isLoading}
					<Loader2 size={16} class="animate-spin" />
				{:else}
					<Send size={16} />
				{/if}
			</Button>
		</div>
		{/if}
	</div>
	{/if}
</div>