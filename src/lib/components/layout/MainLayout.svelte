<script lang="ts">
	import Header from './Header.svelte';
	import FileExplorer from './FileExplorer.svelte';
	import VersionControlPanel from './VersionControlPanel.svelte';
	import MonacoEditor from '../editor/MonacoEditor.svelte';
	import PDFPreview from '../preview/PDFPreview.svelte';
	import EditorHeader from './EditorHeader.svelte';
	import ChatPanel from '../chat/ChatPanel.svelte';
	import BottomChat from '../chat/BottomChat.svelte';
	import WelcomeScreen from '../welcome/WelcomeScreen.svelte';
	import { ResizablePaneGroup, ResizablePane, ResizableHandle } from '$lib/components/ui/resizable';
	import { editorState } from '$lib/stores/editor.js';
	import { useWorkspaceReady } from '$lib/api/hooks';

	let isFileExplorerOpen = $state(true);
	let isVersionControlOpen = $state(false);
	let activeTab = $state<'code' | 'chat'>('code');

	// Check workspace readiness (project selection for desktop)
	const workspaceQuery = useWorkspaceReady();

	$effect(() => {
		isFileExplorerOpen = $editorState.isFileExplorerOpen;
		isVersionControlOpen = $editorState.isVersionControlOpen;
		activeTab = $editorState.activeTab;
	});

	// Reactive values for workspace state  
	let workspaceState = $derived(workspaceQuery());
	let isReady = $derived(workspaceState.isReady);
	let isLoading = $derived(workspaceState.isLoading);
	let needsSetup = $derived(workspaceState.needsSetup);
	let supportsProjects = $derived(workspaceState.supportsProjects);
</script>

{#if needsSetup}
	<!-- Show welcome screen for project selection (desktop only) -->
	<WelcomeScreen />
{:else if isLoading}
	<!-- Loading state -->
	<div class="h-screen flex items-center justify-center bg-background">
		<div class="text-center space-y-4">
			<div class="h-8 w-8 animate-spin rounded-full border-2 border-primary border-t-transparent mx-auto"></div>
			<p class="text-muted-foreground">
				{supportsProjects ? 'Loading project...' : 'Initializing workspace...'}
			</p>
		</div>
	</div>
{:else if isReady}
	<!-- Main editor interface -->
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
{:else}
	<!-- Fallback/error state -->
	<div class="h-screen flex items-center justify-center bg-background">
		<div class="text-center space-y-4">
			<p class="text-destructive">Failed to initialize workspace</p>
			<p class="text-sm text-muted-foreground">Please refresh the page and try again</p>
		</div>
	</div>
{/if}

<style>
	/* Ensure smooth transitions for panel toggles */
	.flex {
		transition: width 0.2s ease-in-out;
	}
</style>