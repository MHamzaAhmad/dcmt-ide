<script lang="ts">
	import { onMount } from 'svelte';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import * as Select from '$lib/components/ui/select';
	import { Badge } from '$lib/components/ui/badge';
	import ChatMessage from './ChatMessage.svelte';
	import { chatStore } from '$lib/stores/chat';
	import { apiStore } from '$lib/stores/api';
	import { modelsAPI } from '$lib/api/models';
	import type { ChatMessage as ChatMessageType, LLMModel } from '$lib/api/types';
	import { Send, Bot, AlertCircle } from '@lucide/svelte';
	import { createQuery } from '@tanstack/svelte-query';
	
	let inputValue = $state('');
	let scrollArea: HTMLDivElement;
	let messages = $state<ChatMessageType[]>([]);
	let selectedModel = $state<LLMModel | null>(null);
	let selectedModelId = $state('');
	let isLoading = $state(false);
	
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
		if (scrollArea && messages.length > 0) {
			requestAnimationFrame(() => {
				if (scrollArea) {
					scrollArea.scrollTop = scrollArea.scrollHeight;
				}
			});
		}
	});
	
	onMount(() => {
		// Add sample messages for demo
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
					content: 'Of course! I\'d be happy to help you with LaTeX. What specific aspect would you like assistance with? Whether it\'s document structure, equations, formatting, or anything else, feel free to ask!',
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
</script>

<div class="h-full flex flex-col bg-background">
	<!-- Chat Messages Area -->
	<div class="flex-1 overflow-hidden">
		<ScrollArea class="h-full">
			<div bind:this={scrollArea} class="h-full overflow-y-auto">
				{#if messages.length === 0}
					<div class="flex flex-col items-center justify-center h-full p-8 text-center">
						<Bot size={48} class="text-muted-foreground mb-4" />
						<h3 class="text-lg font-medium mb-2">Start a conversation</h3>
						<p class="text-sm text-muted-foreground max-w-md">
							Ask questions about your LaTeX document or get help with formatting and syntax.
						</p>
					</div>
				{:else}
					<div class="pb-4">
						{#each messages as message}
							<ChatMessage {message} />
						{/each}
					</div>
				{/if}
				
				{#if isLoading}
					<div class="flex gap-3 p-4 bg-muted/30">
						<div class="w-8 h-8 rounded-full bg-muted flex items-center justify-center">
							<Bot size={16} class="text-muted-foreground animate-pulse" />
						</div>
						<div class="flex-1">
							<div class="flex items-center gap-2">
								<span class="text-sm font-medium">Assistant</span>
								<span class="text-xs text-muted-foreground">• Thinking...</span>
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
	<div class="border-t p-4">
		<div class="flex gap-2 mb-3">
			<Select.Root 
				type="single" 
				bind:value={selectedModelId}
				disabled={$modelsQuery.isLoading}
			>
				<Select.Trigger class="w-[200px] h-9">
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
			
			{#if !$apiStore.isConnected}
				<Badge variant="secondary" class="gap-1">
					<AlertCircle size={12} />
					<span>Offline</span>
				</Badge>
			{/if}
		</div>
		
		<div class="flex gap-2">
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