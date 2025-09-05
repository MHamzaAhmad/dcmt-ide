<script lang="ts">
	import { Tabs, TabsList, TabsTrigger } from '$lib/components/ui/tabs';
	import { editorState } from '$lib/stores/editor';
	import { Code, MessageSquare } from '@lucide/svelte';
	
	let activeTab = $state('code');
	
	$effect(() => {
		activeTab = $editorState.activeTab;
	});
	
	function handleTabChange(value: string) {
		editorState.setActiveTab(value as 'code' | 'chat');
	}
</script>

<div class="h-12 border-b bg-background flex items-center px-3">
	<Tabs value={activeTab} onValueChange={handleTabChange} class="w-auto">
		<TabsList class="h-8 bg-muted/50">
			<TabsTrigger value="code" class="h-7 px-3 text-xs gap-1.5 data-[state=active]:bg-background">
				<Code size={14} />
				<span>Code</span>
			</TabsTrigger>
			<TabsTrigger value="chat" class="h-7 px-3 text-xs gap-1.5 data-[state=active]:bg-background">
				<MessageSquare size={14} />
				<span>Chat</span>
			</TabsTrigger>
		</TabsList>
	</Tabs>
</div>