<script lang="ts">
	import { onMount } from 'svelte';
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import * as Select from '$lib/components/ui/select';
	import { Badge } from '$lib/components/ui/badge';
	import { chatStore } from '$lib/stores/chat';
	import { apiStore } from '$lib/stores/api';
	import { editorState } from '$lib/stores/editor';
	import { modelsAPI } from '$lib/api/models';
	import type { ChatMessage as ChatMessageType, LLMModel } from '$lib/api/types';
	import { Send, Maximize2, User, Bot, AlertCircle } from '@lucide/svelte';
	import { createQuery } from '@tanstack/svelte-query';
	
	let inputValue = $state('');
	let messagesContainer: HTMLDivElement;
	let messages = $state<ChatMessageType[]>([]);
	let selectedModel = $state<LLMModel | null>(null);
	let selectedModelId = $state('');
	let isLoading = $state(false);
	let isExpanded = $state(false);
	
	// Query for fetching models
	const modelsQuery = createQuery({
		queryKey: ['models'],
		queryFn: () => modelsAPI.listModels(),
		staleTime: 5 * 60 * 1000, // 5 minutes
		retry: 2
	});
	
	// Subscribe to chat store
	$effect(() => {
		messages = $chatStore.messages;
		selectedModel = $chatStore.selectedModel;
		isLoading = $chatStore.isLoading;
		// Keep selectedModelId in sync with selectedModel
		if (selectedModel) {
			selectedModelId = selectedModel.id;
		}
	});
	
	// Update available models when query succeeds
	$effect(() => {
		if ($modelsQuery.data?.models) {
			chatStore.setAvailableModels($modelsQuery.data.models);
		}
	});
	
	// Update selected model when ID changes
	$effect(() => {
		if (selectedModelId) {
			const model = $chatStore.availableModels.find(m => m.id === selectedModelId);
			if (model) {
				chatStore.setSelectedModel(model);
			}
		}
	});
	
	// Derive trigger content
	const triggerContent = $derived(
		$modelsQuery.isLoading ? "Loading models..." :
		selectedModel ? selectedModel.name : "Select a model"
	);
	
	// Auto-scroll to bottom when new messages arrive
	$effect(() => {
		if (messagesContainer && messages.length > 0) {
			requestAnimationFrame(() => {
				if (messagesContainer) {
					messagesContainer.scrollTop = messagesContainer.scrollHeight;
				}
			});
		}
	});
	
	onMount(() => {
		// Add sample messages for demo if none exist
		if (messages.length === 0) {
			const sampleMessages: ChatMessageType[] = [
				{
					id: '1',
					role: 'user',
					content: 'Hello! Can you help me with LaTeX?',
					timestamp: new Date(Date.now() - 10000)
				},
				{
					id: '2',
					role: 'assistant',
					content: 'Of course! I\'d be happy to help you with LaTeX. What specific aspect would you like assistance with?',
					timestamp: new Date(Date.now() - 5000),
					model: 'GPT-4'
				}
			];
			
			sampleMessages.forEach(msg => chatStore.addMessage(msg));
		}
	});
	
	function handleSend() {
		if (!inputValue.trim() || !selectedModel) return;
		
		const newMessage: ChatMessageType = {
			id: Date.now().toString(),
			role: 'user',
			content: inputValue.trim(),
			timestamp: new Date()
		};
		
		chatStore.addMessage(newMessage);
		inputValue = '';
		
		// Simulate assistant response for demo
		setTimeout(() => {
			const response: ChatMessageType = {
				id: (Date.now() + 1).toString(),
				role: 'assistant',
				content: `I understand you're asking about "${newMessage.content}". This is a sample response. The actual API integration will provide real responses.`,
				timestamp: new Date(),
				model: selectedModel?.name
			};
			chatStore.addMessage(response);
		}, 1000);
	}
	
	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			handleSend();
		}
	}
	
	function handleFullscreen() {
		editorState.setActiveTab('chat');
	}
	
	function toggleExpanded() {
		isExpanded = !isExpanded;
	}
</script>

<div class="absolute bottom-4 left-4 right-6 bg-background/95 backdrop-blur border rounded-lg shadow-lg {isExpanded ? 'h-96' : 'h-48'} transition-all duration-300 flex flex-col z-10">
	<!-- Header Bar -->
	<div class="flex items-center justify-between px-4 py-2 border-b bg-muted/30">
		<div class="flex items-center gap-2">
			<span class="text-sm font-medium">AI Assistant</span>
			{#if selectedModel}
				<Badge variant="secondary" class="text-xs">{selectedModel.name}</Badge>
			{/if}
			{#if !$apiStore.isConnected}
				<Badge variant="secondary" class="gap-1">
					<AlertCircle size={12} />
					<span>Offline</span>
				</Badge>
			{/if}
		</div>
		<div class="flex items-center gap-1">
			<Button
				onclick={toggleExpanded}
				size="icon"
				variant="ghost"
				class="h-7 w-7"
			>
				<svg 
					class="h-4 w-4 transition-transform {isExpanded ? 'rotate-180' : ''}"
					fill="none"
					stroke="currentColor"
					viewBox="0 0 24 24"
				>
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
				</svg>
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
		</div>
	</div>
	
	<!-- Messages Area -->
	<div class="flex-1 overflow-hidden">
		<div bind:this={messagesContainer} class="h-full overflow-y-auto px-4 py-2 space-y-3">
			{#each messages as message}
				<div class="flex gap-2 {message.role === 'user' ? 'justify-end' : ''}">
					{#if message.role === 'assistant'}
						<div class="w-6 h-6 rounded-full bg-muted flex items-center justify-center flex-shrink-0">
							<Bot size={14} class="text-muted-foreground" />
						</div>
					{/if}
					<div class="max-w-[70%]">
						<div class="{message.role === 'user' ? 'bg-primary text-primary-foreground' : 'bg-muted'} px-3 py-2 rounded-lg text-sm">
							{message.content}
						</div>
						<div class="text-xs text-muted-foreground mt-1 {message.role === 'user' ? 'text-right' : ''}">
							{message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
							{#if message.model}
								<span class="ml-2">• {message.model}</span>
							{/if}
						</div>
					</div>
					{#if message.role === 'user'}
						<div class="w-6 h-6 rounded-full bg-primary flex items-center justify-center flex-shrink-0">
							<User size={14} class="text-primary-foreground" />
						</div>
					{/if}
				</div>
			{/each}
			
			{#if isLoading}
				<div class="flex gap-2">
					<div class="w-6 h-6 rounded-full bg-muted flex items-center justify-center">
						<Bot size={14} class="text-muted-foreground animate-pulse" />
					</div>
					<div class="bg-muted px-3 py-2 rounded-lg">
						<div class="flex gap-1">
							<div class="w-2 h-2 bg-foreground/50 rounded-full animate-bounce" style="animation-delay: 0ms"></div>
							<div class="w-2 h-2 bg-foreground/50 rounded-full animate-bounce" style="animation-delay: 150ms"></div>
							<div class="w-2 h-2 bg-foreground/50 rounded-full animate-bounce" style="animation-delay: 300ms"></div>
						</div>
					</div>
				</div>
			{/if}
		</div>
	</div>
	
	<!-- Input Area -->
	<div class="border-t px-4 py-3">
		<div class="flex gap-2">
			<Select.Root 
				type="single" 
				bind:value={selectedModelId}
				disabled={$modelsQuery.isLoading}
			>
				<Select.Trigger class="w-[160px] h-9">
					{triggerContent}
				</Select.Trigger>
				<Select.Content>
					{#if $modelsQuery.isError}
						<div class="flex items-center gap-2 p-2 text-sm text-destructive">
							<AlertCircle size={14} />
							<span>Failed to load models</span>
						</div>
					{:else if $chatStore.availableModels.length > 0}
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
							No models available
						</div>
					{/if}
				</Select.Content>
			</Select.Root>
			
			<Input
				bind:value={inputValue}
				placeholder={selectedModel ? "Type your message..." : "Select a model first..."}
				disabled={!selectedModel || isLoading}
				onkeydown={handleKeyDown}
				class="flex-1"
			/>
			
			<Button 
				onclick={handleSend}
				disabled={!inputValue.trim() || !selectedModel || isLoading}
				size="icon"
			>
				<Send size={16} />
			</Button>
		</div>
	</div>
</div>