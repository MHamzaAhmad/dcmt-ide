<script lang="ts">
	import { Tabs, TabsList, TabsTrigger } from '$lib/components/ui/tabs';
	import { Code, MessageSquare, Bot } from '@lucide/svelte';
	import { editorState } from '$lib/stores/editor.js';
	import { billingStore } from '$lib/stores/billing';
	import { Tooltip as TooltipRoot, TooltipContent, TooltipTrigger, TooltipProvider } from '$lib/components/ui/tooltip';
	
	interface Props {
		// File information
		activeFilePath?: string | null;
		fileBasename?: string;
		isDirty?: boolean;
		fullPath?: string;
		
		// Status indicators
		isOperationPending?: boolean;
		operationType?: string;
		agentIsModifying?: boolean;
		hasConflict?: boolean;
		lastAgentUpdateTime?: number;
		
		// LaTeX specific
		isLatexFile?: boolean;
		isCompiling?: boolean;
		compilationStatus?: 'success' | 'error' | null;
		
		// Tab configuration
		showTabs?: boolean;
		activeTab?: 'code' | 'chat';
		
		// Callbacks
		onRefreshFile?: () => void;
	}
	
	let {
		activeFilePath = null,
		fileBasename = '',
		isDirty = false,
		fullPath = '',
		isOperationPending = false,
		operationType = '',
		agentIsModifying = false,
		hasConflict = false,
		lastAgentUpdateTime = 0,
		isLatexFile = false,
		isCompiling = false,
		compilationStatus = null,
		showTabs = true,
		activeTab = 'code',
		onRefreshFile
	}: Props = $props();
	
	// Get editor state reactively
	const editorStateValue = $derived($editorState);
	const currentActiveTab = $derived(showTabs ? editorStateValue.activeTab : activeTab);
	const billingState = $derived($billingStore);

	// Prompt limits UI (free: 2 per session; pro: unlimited)
	function hasUnlimitedPrompts(): boolean {
		if (!billingState.isReady) return false;
		return billingState.benefits.some((b) => b.benefit_type === 'unlimited_prompts' || b.description?.toLowerCase().includes('unlimited prompt'));
	}

	function getPromptsUsed(): number {
		if (typeof window === 'undefined') return 0;
		const v = window.sessionStorage.getItem('dcmt-chat-prompts-used');
		const n = v ? parseInt(v, 10) : 0;
		return Number.isFinite(n) && n >= 0 ? n : 0;
	}

	let promptsUsed = $state(0);
	let promptsRemaining = $derived(hasUnlimitedPrompts() ? '∞' : Math.max(0, 2 - promptsUsed));

	if (typeof window !== 'undefined') {
		promptsUsed = getPromptsUsed();
		window.addEventListener('dcmt-prompts-updated', () => {
			promptsUsed = getPromptsUsed();
		});
	}
	
	function handleTabChange(value: string) {
		if (showTabs) {
			editorState.setActiveTab(value as 'code' | 'chat');
		}
	}
	
	// Helper function to get file basename if not provided
	function getBasename(path: string): string {
		return path.split('/').pop() || path;
	}
</script>

<div class="h-8 border-b bg-background flex items-center justify-between px-3">
	<!-- Left side: File info or chat title -->
	<div class="flex items-center gap-2 text-sm">
		{#if currentActiveTab === 'chat'}
			<Bot size={16} class="text-primary" />
			<span class="font-semibold text-foreground">Researgent AI</span>
		{:else if activeFilePath}
			{@const displayBasename = fileBasename || getBasename(fullPath || activeFilePath)}
			<span class="font-medium text-foreground" title={fullPath || activeFilePath}>
				{displayBasename}
			</span>
			{#if isDirty}
				<span class="text-orange-500" title="File has unsaved changes">•</span>
			{/if}
			
			<!-- Status indicators -->
			{#if isOperationPending && operationType}
				{#if operationType === 'writing'}
					<span class="text-xs text-blue-500" title="Saving file...">Saving...</span>
				{:else if operationType === 'reading'}
					<span class="text-xs text-blue-500" title="Loading file...">Loading...</span>
				{/if}
			{/if}
			
			{#if agentIsModifying}
				<span class="text-xs text-blue-500 animate-pulse" title="Agent is updating this file">🤖</span>
			{:else if hasConflict}
				<span class="text-xs text-orange-500" title="Content conflict detected">⚠️</span>
				{#if onRefreshFile}
					<button 
						class="text-xs text-blue-500 hover:text-blue-700 underline"
						onclick={onRefreshFile}
						title="Reload file to resolve conflict"
					>
						Reload
					</button>
				{/if}
			{:else if lastAgentUpdateTime > 0 && (Date.now() - lastAgentUpdateTime < 5000)}
				<span class="text-xs text-green-500" title="Recently updated by agent">✓</span>
			{/if}
			
			{#if isLatexFile}
				{#if isCompiling}
					<span class="text-xs text-blue-500" title="Compiling LaTeX document">⚙️</span>
				{:else if compilationStatus === 'success'}
					<span class="text-xs text-green-500" title="LaTeX compiled successfully">✓</span>
				{:else if compilationStatus === 'error'}
					<span class="text-xs text-red-500" title="LaTeX compilation failed">✗</span>
				{/if}
			{/if}
		{:else}
			<span class="text-muted-foreground">No file open</span>
		{/if}
	</div>
	
	<!-- Right side: Tabs + prompts badge -->
	{#if showTabs}
		<div class="flex items-center gap-2">
			<Tabs value={currentActiveTab} onValueChange={handleTabChange} class="w-auto">
				<TabsList class="h-6 bg-muted/50">
					<TabsTrigger value="code" class="h-5 px-2 text-xs gap-1 data-[state=active]:bg-background">
						<Code size={12} />
						<span>Code</span>
					</TabsTrigger>
					<TabsTrigger value="chat" class="h-5 px-2 text-xs gap-1 data-[state=active]:bg-background">
						<MessageSquare size={12} />
						<span>Chat</span>
					</TabsTrigger>
				</TabsList>
			</Tabs>

			<!-- Prompts remaining circular badge with tooltip -->
			<TooltipProvider>
				<TooltipRoot>
					<TooltipTrigger>
						<div class="w-6 h-6 rounded-full border flex items-center justify-center text-xs select-none">
							{billingState.isReady ? promptsRemaining : '–'}
						</div>
					</TooltipTrigger>
					<TooltipContent>
						{#if billingState.isReady}
							{hasUnlimitedPrompts() ? 'Unlimited prompts' : (promptsRemaining === 0 ? 'No prompts remaining' : `${promptsRemaining} prompt${promptsRemaining === 1 ? '' : 's'} remaining`)}
						{:else}
							Loading usage…
						{/if}
					</TooltipContent>
				</TooltipRoot>
			</TooltipProvider>
		</div>
	{/if}
</div>