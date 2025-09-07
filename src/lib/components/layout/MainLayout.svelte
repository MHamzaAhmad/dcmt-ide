<script lang="ts">
	import Header from './Header.svelte';
	import FileExplorer from './FileExplorer.svelte';
	import VersionControlPanel from './VersionControlPanel.svelte';
	import MonacoEditor from '../editor/MonacoEditor.svelte';
	import PDFPreview from '../preview/PDFPreview.svelte';
	import EditorHeader from './EditorHeader.svelte';
	import ChatPanel from '../chat/ChatPanel.svelte';
	import WelcomeScreen from '../welcome/WelcomeScreen.svelte';
	import { ResizablePaneGroup, ResizablePane, ResizableHandle } from '$lib/components/ui/resizable';
	import { editorState } from '$lib/stores/editor.js';
	import { useWorkspaceReady } from '$lib/api/hooks';
	import { useFindMainLatexFile } from '$lib/api/hooks/useLaTeX';
	import { openFiles } from '$lib/stores/files.js';
	import { fileSystemApi } from '$lib/api/adapters';

	let isFileExplorerOpen = $state(true);
	let isVersionControlOpen = $state(false);
	let activeTab = $state<'code' | 'chat'>('code');

	import { isTauri } from '$lib/utils/platform';
	
	// Check workspace readiness (project selection for desktop)
	const currentProjectQuery = useWorkspaceReady();
	
	// Find main LaTeX file on startup
	const mainLatexFileQuery = useFindMainLatexFile();

	$effect(() => {
		isFileExplorerOpen = $editorState.isFileExplorerOpen;
		isVersionControlOpen = $editorState.isVersionControlOpen;
		activeTab = $editorState.activeTab;
	});

	// Auto-load main LaTeX file and PDF on startup
	$effect(() => {
		// Debug: Log all condition values
		console.log('MainLayout effect conditions:', {
			mainLatexData: $mainLatexFileQuery.data,
			currentProjectData: $currentProjectQuery.data,
			isReady,
			supportsProjects,
			openFilesLength: $openFiles.length,
			platform: isTauri() ? 'desktop' : 'web'
		});
		
		// For web: don't require currentProjectQuery.data, for desktop: require it
		const shouldProceed = $mainLatexFileQuery.data && isReady && 
			(supportsProjects ? $currentProjectQuery.data : true);
		
		console.log('🔍 Condition evaluation:', {
			shouldProceed,
			hasMainLatexData: !!$mainLatexFileQuery.data,
			isReady,
			projectCondition: supportsProjects ? !!$currentProjectQuery.data : true
		});
			
		if (shouldProceed) {
			const mainTexFile = $mainLatexFileQuery.data;
			console.log('Found main LaTeX file:', mainTexFile);
			
			// Auto-open the main .tex file if no files are currently open
			if ($openFiles.length === 0) {
				autoLoadMainFiles(mainTexFile);
			}
		}
	});

	async function autoLoadMainFiles(mainTexPath: string) {
		console.log('🚀 autoLoadMainFiles called with path:', mainTexPath);
		console.log('Platform:', isTauri() ? 'desktop' : 'web');
		
		try {
			// Open the main .tex file in the editor
			console.log('Auto-loading main LaTeX file:', mainTexPath);
			
			// Read the file content
			console.log('🔍 About to call fileSystemApi.readFileContent...');
			const fileContent = await fileSystemApi.readFileContent(mainTexPath);
			console.log('✅ File content loaded successfully:', {
				path: fileContent.path,
				size: fileContent.content.length,
				modified: fileContent.modified
			});
			
			// Add to open files
			console.log('📁 Adding file to open files...');
			openFiles.openFile({
				id: mainTexPath,
				path: mainTexPath,
				name: mainTexPath.split('/').pop() || mainTexPath,
				content: fileContent.content
			});
			
			// Set as active file
			console.log('🎯 Setting as active file...');
			editorState.setActiveFile(mainTexPath);
			
			// PDF loading is now handled by PDFPreview component automatically
			
			console.log('✨ Main LaTeX file and associated files loaded successfully');
		} catch (error) {
			console.error('❌ Failed to auto-load main files:', error);
			console.error('Error details:', {
				message: error instanceof Error ? error.message : 'Unknown error',
				stack: error instanceof Error ? error.stack : 'No stack trace'
			});
		}
	}

	// Reactive values for workspace state using the query store
	let hasProject = $derived(!!$currentProjectQuery.data);
	let isLoading = $derived($currentProjectQuery.isLoading);
	let supportsProjects = $derived(isTauri());
	
	// For web, workspace is always ready (uses predefined /workspace)
	// For desktop, workspace is ready when project is selected
	let isReady = $derived(supportsProjects ? hasProject && !isLoading : true);
	let needsSetup = $derived(supportsProjects && !hasProject && !isLoading);

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
						
						<!-- Floating Chat (only visible in code view) -->
						{#if activeTab === 'code'}
							<ChatPanel mode="floating" />
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