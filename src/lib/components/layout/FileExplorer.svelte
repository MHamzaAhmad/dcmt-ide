<script lang="ts">
	import { TreeView, TreeViewFile, TreeViewFolder } from '$lib/components/ui/tree-view';
	import { File, Folder, FolderOpen, FileText, Loader2, AlertCircle, FilePlus, FolderPlus, Edit, Trash2 } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
	import { openFiles } from '$lib/stores/files.js';
	import { editorState } from '$lib/stores/editor.js';
	import { useDirectoryTree, useFileContent, useCreateFile, useDeleteFile, useRenameFile } from '$lib/api/hooks';

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
	
	
	// File mutations
	const createFileMutation = useCreateFile();
	const deleteFileMutation = useDeleteFile();
	const renameFileMutation = useRenameFile();

	let renamingItem = $state<string | null>(null);
	let renameValue = $state<string>('');

	// Convert API response to our FileNode format
	function convertFileInfoToNode(fileInfo: any, parentPath: string = ''): FileNode {
		const fullPath = parentPath ? `${parentPath}/${fileInfo.name}` : fileInfo.name;
		
		return {
			id: fullPath,
			name: fileInfo.name,
			path: fullPath,
			type: fileInfo.file_type === 'Directory' ? 'directory' : 'file',
			children: fileInfo.children?.map((child: any) => convertFileInfoToNode(child, fullPath))
		};
	}

	// Reactive file tree conversion
	let files = $derived($directoryQuery.data ? 
		($directoryQuery.data.children?.map((child: any) => convertFileInfoToNode(child)) || []) : 
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

	// Handler for creating a new file
	async function handleCreateFile() {
		try {
			await $createFileMutation.mutateAsync({
				path: 'untitled.txt',
				content: '',
				isDirectory: false
			});
		} catch (error) {
			console.error('Failed to create file:', error);
		}
	}

	// Handler for creating a new folder
	async function handleCreateFolder() {
		try {
			await $createFileMutation.mutateAsync({
				path: 'New Folder',
				isDirectory: true
			});
		} catch (error) {
			console.error('Failed to create folder:', error);
		}
	}

	// Context menu handlers
	async function handleCreateFileInFolder(folderPath: string, isDirectory = false) {
		try {
			const itemName = isDirectory ? 'New Folder' : 'untitled.txt';
			const fullPath = folderPath ? `${folderPath}/${itemName}` : itemName;
			await $createFileMutation.mutateAsync({
				path: fullPath,
				content: isDirectory ? undefined : '',
				isDirectory: isDirectory
			});
		} catch (error) {
			console.error(`Failed to create ${isDirectory ? 'folder' : 'file'}:`, error);
		}
	}


	async function handleDeleteItem(item: FileNode) {
		try {
			await $deleteFileMutation.mutateAsync(item.path);
		} catch (error) {
			console.error('Failed to delete item:', error);
		}
	}

	function handleRenameStart(item: FileNode) {
		renamingItem = item.path;
		renameValue = item.name;
	}

	async function handleRenameConfirm(item: FileNode) {
		if (renameValue.trim() && renameValue !== item.name) {
			try {
				const parentPath = item.path.split('/').slice(0, -1).join('/');
				const newPath = parentPath ? `${parentPath}/${renameValue}` : renameValue;
				await $renameFileMutation.mutateAsync({
					oldPath: item.path,
					newPath: newPath
				});
			} catch (error) {
				console.error('Failed to rename item:', error);
			}
		}
		renamingItem = null;
		renameValue = '';
	}

	function handleRenameCancel() {
		renamingItem = null;
		renameValue = '';
	}


</script>

{#snippet treeNodeSnippet(node: FileNode)}
	{#if node.type === 'directory'}
		<ContextMenu.Root>
			<ContextMenu.Trigger class="w-full">
				<TreeViewFolder 
					name={renamingItem === node.path ? '' : node.name}
					class="text-sm hover:bg-accent"
				>
				{#snippet icon({ open })}
					{#if renamingItem === node.path}
						<input 
							bind:value={renameValue}
							class="text-sm bg-background border rounded px-1 py-0 w-full"
							onkeydown={(e) => {
								if (e.key === 'Enter') handleRenameConfirm(node);
								else if (e.key === 'Escape') handleRenameCancel();
							}}
							onblur={() => handleRenameConfirm(node)}
							onfocus={(e) => (e.target as HTMLInputElement)?.select()}
						/>
					{:else}
						{#if open}
							<FolderOpen size={16} class="text-blue-500" />
						{:else}
							<Folder size={16} class="text-blue-500" />
						{/if}
					{/if}
				{/snippet}
				{#if node.children && node.children.length > 0}
					{#each node.children as child}
						{@render treeNodeSnippet(child)}
					{/each}
				{/if}
			</TreeViewFolder>
			</ContextMenu.Trigger>
			<ContextMenu.Content class="w-48">
				<ContextMenu.Item onclick={() => handleCreateFileInFolder(node.path)}>
					<FilePlus size={16} />
					New File
				</ContextMenu.Item>
				<ContextMenu.Item onclick={() => handleCreateFileInFolder(node.path, true)}>
					<FolderPlus size={16} />
					New Folder
				</ContextMenu.Item>
				<ContextMenu.Separator />
				<ContextMenu.Item onclick={() => handleRenameStart(node)}>
					<Edit size={16} />
					Rename
				</ContextMenu.Item>
				<ContextMenu.Item variant="destructive" onclick={() => handleDeleteItem(node)}>
					<Trash2 size={16} />
					Delete
				</ContextMenu.Item>
			</ContextMenu.Content>
		</ContextMenu.Root>
	{:else}
		<ContextMenu.Root>
			<ContextMenu.Trigger class="w-full">
				<TreeViewFile 
					name={renamingItem === node.path ? '' : node.name}
					class="text-sm hover:bg-accent"
					onclick={() => handleFileClick(node)}
				>
				{#snippet icon({ name })}
					{#if renamingItem === node.path}
						<input 
							bind:value={renameValue}
							class="text-sm bg-background border rounded px-1 py-0 w-full"
							onkeydown={(e) => {
								if (e.key === 'Enter') handleRenameConfirm(node);
								else if (e.key === 'Escape') handleRenameCancel();
							}}
							onblur={() => handleRenameConfirm(node)}
							onfocus={(e) => (e.target as HTMLInputElement)?.select()}
						/>
					{:else}
						{@const IconComponent = getFileIcon(name)}
						<IconComponent size={16} class="text-muted-foreground" />
					{/if}
				{/snippet}
			</TreeViewFile>
			</ContextMenu.Trigger>
			<ContextMenu.Content class="w-48">
				<ContextMenu.Item onclick={() => handleRenameStart(node)}>
					<Edit size={16} />
					Rename
				</ContextMenu.Item>
				<ContextMenu.Item variant="destructive" onclick={() => handleDeleteItem(node)}>
					<Trash2 size={16} />
					Delete
				</ContextMenu.Item>
			</ContextMenu.Content>
		</ContextMenu.Root>
	{/if}
{/snippet}

<div class="h-full flex flex-col">
	<div class="p-3 border-b flex items-center justify-between">
		<h3 class="text-sm font-medium">Explorer</h3>
		<div class="flex items-center gap-1">
			<Button 
				variant="ghost" 
				size="icon" 
				class="h-6 w-6" 
				onclick={handleCreateFile}
				disabled={$createFileMutation.isPending}
			>
				<FilePlus size={16} />
			</Button>
			<Button 
				variant="ghost" 
				size="icon" 
				class="h-6 w-6" 
				onclick={handleCreateFolder}
				disabled={$createFileMutation.isPending}
			>
				<FolderPlus size={16} />
			</Button>
		</div>
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
			<!-- File tree with context menu for empty space -->
			<ContextMenu.Root>
				<ContextMenu.Trigger class="w-full h-full">
					<TreeView class="w-full">
						{#each files as node}
							{@render treeNodeSnippet(node)}
						{/each}
					</TreeView>
				</ContextMenu.Trigger>
				<ContextMenu.Content class="w-48">
					<ContextMenu.Item onclick={() => handleCreateFile()}>
						<FilePlus size={16} />
						New File
					</ContextMenu.Item>
					<ContextMenu.Item onclick={() => handleCreateFolder()}>
						<FolderPlus size={16} />
						New Folder
					</ContextMenu.Item>
				</ContextMenu.Content>
			</ContextMenu.Root>
		{:else}
			<!-- Empty state -->
			<div class="p-4 text-center text-muted-foreground text-sm">
				No files found
			</div>
		{/if}
	</div>
</div>