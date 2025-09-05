<script lang="ts">
	import Header from './Header.svelte';
	import FileExplorer from './FileExplorer.svelte';
	import VersionControlPanel from './VersionControlPanel.svelte';
	import MonacoEditor from '../editor/MonacoEditor.svelte';
	import PDFPreview from '../preview/PDFPreview.svelte';
	import EditorHeader from './EditorHeader.svelte';
	import ChatPanel from '../chat/ChatPanel.svelte';
	import BottomChat from '../chat/BottomChat.svelte';
	import { ResizablePaneGroup, ResizablePane, ResizableHandle } from '$lib/components/ui/resizable';
	import { editorState } from '$lib/stores/editor.js';

	let isFileExplorerOpen = $state(true);
	let isVersionControlOpen = $state(false);
	let activeTab = $state<'code' | 'chat'>('code');

	$effect(() => {
		isFileExplorerOpen = $editorState.isFileExplorerOpen;
		isVersionControlOpen = $editorState.isVersionControlOpen;
		activeTab = $editorState.activeTab;
	});
</script>

<div class="h-screen flex flex-col bg-background">
	<!-- Header -->
	<Header />
	
	<!-- Main Content Area -->
	<div class="flex-1 overflow-hidden">
		<ResizablePaneGroup direction="horizontal" class="h-full">
			<!-- Left Sidebar -->
			{#if isFileExplorerOpen || isVersionControlOpen}
				<ResizablePane defaultSize={20} minSize={15} maxSize={40} class="bg-background">
					{#if isFileExplorerOpen && isVersionControlOpen}
						<!-- Both panels open - split view -->
						<ResizablePaneGroup direction="vertical" class="h-full">
							<ResizablePane defaultSize={50} minSize={30} class="border-b">
								<FileExplorer />
							</ResizablePane>
							<ResizableHandle />
							<ResizablePane defaultSize={50} minSize={30}>
								<VersionControlPanel />
							</ResizablePane>
						</ResizablePaneGroup>
					{:else if isFileExplorerOpen}
						<!-- Only file explorer -->
						<FileExplorer />
					{:else if isVersionControlOpen}
						<!-- Only version control -->
						<VersionControlPanel />
					{/if}
				</ResizablePane>
				<ResizableHandle />
			{/if}

			<!-- Editor Panel -->
			<ResizablePane defaultSize={40} minSize={25} class="border-r">
				<div class="h-full flex flex-col relative overflow-visible">
					<EditorHeader />
					<div class="flex-1 overflow-hidden">
						{#if activeTab === 'code'}
							<MonacoEditor />
						{:else}
							<ChatPanel />
						{/if}
					</div>
					
					<!-- Floating Bottom Chat (only visible in code view) -->
					{#if activeTab === 'code'}
						<BottomChat />
					{/if}
				</div>
			</ResizablePane>
			<ResizableHandle />
			
			<!-- PDF Preview Panel -->
			<ResizablePane defaultSize={40} minSize={25}>
				<PDFPreview />
			</ResizablePane>
		</ResizablePaneGroup>
	</div>
</div>

<style>
	/* Ensure smooth transitions for panel toggles */
	.flex {
		transition: width 0.2s ease-in-out;
	}
</style>