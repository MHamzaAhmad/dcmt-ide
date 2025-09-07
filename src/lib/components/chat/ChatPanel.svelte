<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import * as Select from '$lib/components/ui/select';
	import { Badge } from '$lib/components/ui/badge';
	import ChatMessage from './ChatMessage.svelte';
	import ToolExecution from './ToolExecution.svelte';
	import { chatStore } from '$lib/stores/chat';
	import { agentStore } from '$lib/stores/agent';
	import { apiStore } from '$lib/stores/api';
	import { editorState } from '$lib/stores/editor';
	import { modelsAPI } from '$lib/api/models';
	import type { ChatMessage as ChatMessageType, LLMModel, AgentChatMessage, LiteLLMModel } from '$lib/api/types';
	import { Send, Bot, AlertCircle, Wifi, WifiOff, Settings, Loader2, Maximize2, Minimize2, ChevronDown, ChevronUp } from '@lucide/svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	
	interface Props {
		mode?: 'docked' | 'floating';
	}
	
	let { mode = 'docked' }: Props = $props();
	
	let inputValue = $state('');
	let scrollArea = $state<HTMLDivElement>();
	let messages = $state<AgentChatMessage[]>([]);
	let selectedModel = $state<LiteLLMModel | null>(null);
	let selectedModelId = $state('');
	let isLoading = $state(false);
	let isAgentMode = $state(true); // Toggle between agent and simple chat
	let isExpanded = $state(false); // For floating mode expand/collapse
	let isMinimized = $state(false); // For floating mode minimize
	
	// Agent state
	let agentAvailable = $state(false);
	let agentConnected = $state(false);
	let agentError = $state<string | null>(null);
	let toolResults = $state(new Map());
	let streamingContent = $state('');
	let isProcessing = $state(false);
	
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
			agentConnected = agentState.isConnected;
			agentError = agentState.connectionError;
			messages = agentState.messages;
			selectedModel = agentState.selectedModel;
			toolResults = agentState.activeToolResults;
			streamingContent = agentState.streamingContent;
			isProcessing = agentState.isProcessing;
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
	
	// Auto-scroll to bottom when new messages arrive
	$effect(() => {
		if (scrollArea && (messages.length > 0 || toolResults.size > 0 || streamingContent)) {
			requestAnimationFrame(() => {
				if (scrollArea) {
					scrollArea.scrollTop = scrollArea.scrollHeight;
				}
			});
		}
	});
	
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
	
	function toggleAgentMode() {
		isAgentMode = !isAgentMode;
		// Clear messages when switching modes
		if (isAgentMode) {
			// Pass queryClient when re-initializing
			agentStore.setQueryClient(queryClient);
			agentStore.initialize();
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
					{#if !isMinimized && selectedModel}
						<Badge variant="secondary" class="text-xs">{selectedModel.id}</Badge>
					{/if}
				{:else}
					<Bot size={20} class="text-primary" />
					<h2 class="text-lg font-semibold">AI Assistant</h2>
					{#if isAgentMode}
						<Badge variant="default" class="gap-1">
							{#if agentConnected}
								<Wifi size={12} />
							{:else}
								<WifiOff size={12} />
							{/if}
							Agent Mode
						</Badge>
					{:else}
						<Badge variant="outline">Simple Chat</Badge>
					{/if}
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
					<Button variant="ghost" size="sm" onclick={toggleAgentMode}>
						{isAgentMode ? 'Simple' : 'Agent'}
					</Button>
					<Button variant="ghost" size="sm" onclick={handleClearSession}>
						Clear
					</Button>
				{/if}
			</div>
		</div>
		
		{#if isAgentMode && agentError}
			<div class="mt-2 p-2 bg-destructive/10 border border-destructive/20 rounded text-sm text-destructive">
				<div class="flex items-center gap-2">
					<AlertCircle size={14} />
					<span>Agent Error: {agentError}</span>
				</div>
			</div>
		{/if}
	</div>

	<!-- Chat Messages Area -->
	{#if !isMinimized}
	<div class="flex-1 overflow-hidden">
		<ScrollArea class="h-full">
			<div bind:this={scrollArea} class="h-full overflow-y-auto">
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
					</div>
				{/if}
				
				<!-- Tool Execution Display -->
				{#if isAgentMode && toolResults.size > 0}
					<div class="px-4">
						<ToolExecution {toolResults} />
					</div>
				{/if}
				
				
				<!-- Processing Indicator (only show when no streaming message) -->
				{#if (isAgentMode ? (isProcessing && !streamingContent) : isLoading)}
					<div class="flex gap-3 p-4 bg-muted/30">
						<div class="w-8 h-8 rounded-full bg-muted flex items-center justify-center">
							<Bot size={16} class="text-muted-foreground animate-pulse" />
						</div>
						<div class="flex-1">
							<div class="flex items-center gap-2">
								<span class="text-sm font-medium">Assistant</span>
								<span class="text-xs text-muted-foreground">
									• {isAgentMode && toolResults.size > 0 ? 'Using tools...' : 'Processing...'}
								</span>
							</div>
							<div class="flex gap-1 mt-2">
								<div class="w-2 h-2 bg-primary/50 rounded-full animate-bounce" style="animation-delay: 0ms"></div>
								<div class="w-2 h-2 bg-primary/50 rounded-full animate-bounce" style="animation-delay: 150ms"></div>
								<div class="w-2 h-2 bg-primary/50 rounded-full animate-bounce" style="animation-delay: 300ms"></div>
							</div>
						</div>
					</div>
				{/if}
			</div>
		</ScrollArea>
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
							<Select.Label>LiteLLM Models</Select.Label>
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
			
			<!-- Status Indicators -->
			{#if mode === 'docked'}
			<div class="flex items-center gap-1">
				{#if isAgentMode}
					{#if !agentAvailable}
						<Badge variant="destructive" class="gap-1">
							<AlertCircle size={12} />
							<span>No Project</span>
						</Badge>
					{:else if !agentConnected}
						<Badge variant="secondary" class="gap-1">
							<WifiOff size={12} />
							<span>Disconnected</span>
						</Badge>
					{:else}
						<Badge variant="default" class="gap-1">
							<Wifi size={12} />
							<span>Connected</span>
						</Badge>
					{/if}
				{:else if !$apiStore.isConnected}
					<Badge variant="secondary" class="gap-1">
						<AlertCircle size={12} />
						<span>Offline</span>
					</Badge>
				{/if}
			</div>
			{/if}
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
		<div class="flex gap-2">
			<Input
				bind:value={inputValue}
				placeholder={selectedModel ? "Ask me to help with your LaTeX project..." : "Select a model first..."}
				disabled={!selectedModel || (isAgentMode ? isProcessing : isLoading)}
				onkeydown={handleKeyDown}
				class="flex-1"
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