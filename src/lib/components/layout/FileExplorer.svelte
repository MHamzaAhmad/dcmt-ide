<script lang="ts">
	import { TreeView, TreeViewFile, TreeViewFolder } from '$lib/components/ui/tree-view';
	import { File, Folder, FolderOpen, FileText, Loader2, AlertCircle } from '@lucide/svelte';
	import { openFiles } from '$lib/stores/files.js';
	import { editorState } from '$lib/stores/editor.js';
	import { useDirectoryTree, useFileContent, useAutoRefresh } from '$lib/api/hooks';
	import type { FileInfo } from '$lib/api/types';

	// Convert FileInfo to FileNode format for compatibility with existing components
	interface FileNode {
		id: string;
		name: string;
		path: string;
		type: 'file' | 'directory';
		children?: FileNode[];
	}

	// Use our unified file system API
	const directoryQuery = useDirectoryTree('');
	let selectedFilePath = $state<string>('');
	
	// Create reactive file content query
	let fileContentQuery = $derived(useFileContent(selectedFilePath, !!selectedFilePath));
	
	// Enable auto-refresh for real-time file watching
	const fileWatcher = useAutoRefresh(true);

	// Convert API FileInfo to our FileNode format
	function convertFileInfoToNode(fileInfo: FileInfo, parentPath: string = ''): FileNode {
		const fullPath = parentPath ? `${parentPath}/${fileInfo.name}` : fileInfo.name;
		
		return {
			id: fullPath,
			name: fileInfo.name,
			path: fullPath,
			type: fileInfo.file_type === 'Directory' ? 'directory' : 'file',
			children: fileInfo.children?.map(child => convertFileInfoToNode(child, fullPath))
		};
	}

	// Reactive file tree conversion
	let files = $derived($directoryQuery.data ? 
		($directoryQuery.data.children?.map(child => convertFileInfoToNode(child)) || []) : 
		[]);

	async function handleFileClick(file: FileNode) {
		if (file.type === 'file') {
			// Set selected file path to trigger content loading
			selectedFilePath = file.path;
			
			// For now, open with placeholder content - the query will update it
			openFiles.openFile({
				id: file.id,
				name: file.name,
				path: file.path,
				content: 'Loading...' // This will be updated by the file content query
			});
			
			editorState.setActiveFile(file.id);
		}
	}

	// Watch for file content changes and update the opened file
	$effect(() => {
		if (selectedFilePath && $fileContentQuery.data) {
			// Update the opened file with the loaded content
			const fileId = selectedFilePath;
			openFiles.updateFileContent(fileId, $fileContentQuery.data.content);
		}
	});

	function getFileIcon(fileName: string) {
		if (fileName.endsWith('.tex')) {
			return FileText;
		}
		return File;
	}
</script>

<div class="h-full flex flex-col">
	<div class="p-3 border-b">
		<h3 class="text-sm font-medium">Explorer</h3>
	</div>
	
	<div class="flex-1 overflow-auto p-2">
		{#if $directoryQuery.isPending}
			<!-- Loading state -->
			<div class="p-4 text-center">
				<Loader2 class="h-6 w-6 animate-spin mx-auto mb-2" />
				<p class="text-sm text-muted-foreground">Loading files...</p>
			</div>
		{:else if $directoryQuery.isError}
			<!-- Error state -->
			<div class="p-4 text-center">
				<AlertCircle class="h-6 w-6 text-destructive mx-auto mb-2" />
				<p class="text-sm text-destructive">Failed to load files</p>
				<p class="text-xs text-muted-foreground mt-1">
					{$directoryQuery.error?.message || 'Unknown error'}
				</p>
			</div>
		{:else if files.length > 0}
			<!-- File tree -->
			<TreeView class="w-full">
				{#each files as node}
					<TreeViewFile 
						name={node.name}
						class="text-sm hover:bg-accent"
						onclick={() => handleFileClick(node)}
					>
						{#snippet icon({ name })}
							{#if node.type === 'directory'}
								<Folder size={16} class="text-blue-500" />
							{:else}
								{@const IconComponent = getFileIcon(name)}
								<IconComponent size={16} class="text-muted-foreground" />
							{/if}
						{/snippet}
					</TreeViewFile>
				{/each}
			</TreeView>
		{:else}
			<!-- Empty state -->
			<div class="p-4 text-center text-muted-foreground text-sm">
				No files found
			</div>
		{/if}
	</div>
</div>