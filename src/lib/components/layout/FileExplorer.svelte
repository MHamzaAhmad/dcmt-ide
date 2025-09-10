<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { TreeView, TreeViewFile, TreeViewFolder } from '$lib/components/ui/tree-view';
	import { File, Folder, FolderOpen, FileText, Loader2, AlertCircle, FilePlus, FolderPlus, Edit, Trash2 } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
	import { editorState } from '$lib/stores/editor.js';
	import { workspaceStore } from '$lib/stores/workspace';
	import { useDirectoryTree, useCreateFile, useDeleteFile, useRenameFile } from '$lib/api/hooks';
	import { useFileWatcher } from '$lib/api/hooks/useFileWatcher';
	import { fileSystemApi } from '$lib/api/adapters';

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
	let isLoadingFile = $state<boolean>(false);
	
	// Initialize file watcher manually with controlled lifecycle
	const fileWatcher = useFileWatcher(undefined, true);
	
	
	// File mutations
	const createFileMutation = useCreateFile();
	const deleteFileMutation = useDeleteFile();
	const renameFileMutation = useRenameFile();

	let renamingItem = $state<string | null>(null);
	let renameValue = $state<string>('');
	let creatingItem = $state<{path: string, isDirectory: boolean, parentPath: string, placeholder: FileNode} | null>(null);

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

	// Generate unique name with timestamp
	function generateUniqueName(baseName: string, isDirectory: boolean = false): string {
		const timestamp = Date.now();
		if (isDirectory) {
			return `${baseName}_${timestamp}`;
		}
		const parts = baseName.split('.');
		if (parts.length > 1) {
			const name = parts.slice(0, -1).join('.');
			const ext = parts[parts.length - 1];
			return `${name}_${timestamp}.${ext}`;
		}
		return `${baseName}_${timestamp}`;
	}

	// Add placeholder to file tree
	function addPlaceholderToTree(nodes: FileNode[], placeholder: FileNode, parentPath: string): FileNode[] {
		if (!parentPath) {
			// Add to root level
			return [...nodes, placeholder];
		}

		return nodes.map(node => {
			if (node.path === parentPath && node.type === 'directory') {
				// Add placeholder to this directory's children
				return {
					...node,
					children: [...(node.children || []), placeholder]
				};
			} else if (node.children) {
				// Recursively search in children
				return {
					...node,
					children: addPlaceholderToTree(node.children, placeholder, parentPath)
				};
			}
			return node;
		});
	}

	// Base file tree from API
	let baseFiles = $derived($directoryQuery.data ? 
		($directoryQuery.data.children?.map((child: any) => convertFileInfoToNode(child)) || []) : 
		[]);

	// Current placeholder item - use the stored placeholder to avoid timestamp mismatches
	let currentPlaceholder = $derived(creatingItem ? creatingItem.placeholder : null);

	// Combined files with placeholder
	let files = $derived.by(() => {
		let result = [...baseFiles];
		
		if (currentPlaceholder && creatingItem) {
			result = addPlaceholderToTree(result, currentPlaceholder, creatingItem.parentPath);
		}

		return result;
	});

	// Create placeholder item with stable ID
	function createPlaceholderItem(parentPath: string = '', isDirectory: boolean = false): FileNode {
		const baseName = isDirectory ? 'New Folder' : 'untitled.txt';
		const uniqueName = generateUniqueName(baseName, isDirectory);
		const fullPath = parentPath ? `${parentPath}/${uniqueName}` : uniqueName;
		
		// Use timestamp for stable ID to prevent re-render issues
		const timestamp = Date.now();
		const placeholderId = `placeholder_${timestamp}`;
		
		const placeholder: FileNode = {
			id: placeholderId,
			name: uniqueName,
			path: fullPath,
			type: isDirectory ? 'directory' : 'file',
			children: isDirectory ? [] : undefined
		};

		return placeholder;
	}

	// Consolidated state reset function
	function resetAllStates() {
		creatingItem = null;
		renamingItem = null;
		renameValue = '';
	}

	// Start creation process
	function startCreation(parentPath: string = '', isDirectory: boolean = false) {
		// Prevent multiple simultaneous creations
		if (creatingItem || $createFileMutation.isPending) {
			return;
		}

		// Reset any existing states first
		resetAllStates();
		
		// Create the placeholder and set all states synchronously
		const placeholder = createPlaceholderItem(parentPath, isDirectory);
		
		// Set all states at once to avoid race conditions
		creatingItem = { 
			path: placeholder.path, 
			isDirectory, 
			parentPath,
			placeholder
		};
		renamingItem = placeholder.path;
		renameValue = placeholder.name;
	}

	// Cancel creation - reset states (placeholder will be removed automatically)
	function cancelCreation() {
		resetAllStates();
	}

	// Confirm creation - create the actual file/folder with the chosen name
	async function confirmCreation() {
		if (!creatingItem || !renameValue.trim()) {
			cancelCreation();
			return;
		}

		try {
			const fullPath = creatingItem.parentPath ? 
				`${creatingItem.parentPath}/${renameValue}` : 
				renameValue;
			
			await $createFileMutation.mutateAsync({
				path: fullPath,
				content: creatingItem.isDirectory ? undefined : '',
				isDirectory: creatingItem.isDirectory
			});

			// Reset state after successful creation
			resetAllStates();
		} catch (error) {
			console.error('Failed to create item:', error);
			// Keep creation state active so user can retry with different name
			// Only reset rename states, not the creation placeholder
			renamingItem = null;
			renameValue = '';
		}
	}

	async function handleFileClick(file: FileNode) {
		if (file.type === 'file' && !file.id.startsWith('placeholder_')) {
			// Prevent multiple clicks while loading
			if (isLoadingFile) return;
			
			// Set loading state
			isLoadingFile = true;
			
			try {
				// Use workspaceStore to open the file
				await workspaceStore.openFile(file.path);
				
				console.log('File opened via workspaceStore:', file.path);
			} catch (error) {
				console.error('Failed to load file:', error);
				// Could show a toast notification here
			} finally {
				// Reset loading state
				isLoadingFile = false;
			}
		}
	}

	// Click-outside detection for cancelling creation
	$effect(() => {
		if (creatingItem) {
			const handleClickOutside = (event: MouseEvent) => {
				const target = event.target as HTMLElement;
				// Check if click is outside the rename input and not on UI controls
				if (!target.closest('input') && 
					!target.closest('[data-tree-view]') && 
					!target.closest('button') &&
					!target.closest('[role="menu"]') &&
					!target.closest('.tree-view')) {
					cancelCreation();
				}
			};

			// Small delay to avoid immediate cancellation when creating
			const timeoutId = setTimeout(() => {
				document.addEventListener('click', handleClickOutside);
			}, 100);

			return () => {
				clearTimeout(timeoutId);
				document.removeEventListener('click', handleClickOutside);
			};
		}
	});

	// Auto-focus input when starting creation or rename
	$effect(() => {
		if (renamingItem) {
			// Small delay to ensure DOM is updated
			setTimeout(() => {
				const inputs = document.querySelectorAll('input');
				const input = Array.from(inputs).find(inp => inp.value === renameValue);
				if (input) {
					input.focus();
					input.select();
				}
			}, 10);
		}
	});

	function getFileIcon(fileName: string) {
		if (fileName.endsWith('.tex')) {
			return FileText;
		}
		return File;
	}

	// Handler for creating a new file
	function handleCreateFile() {
		startCreation('', false);
	}

	// Handler for creating a new folder
	function handleCreateFolder() {
		startCreation('', true);
	}

	// Context menu handlers
	function handleCreateFileInFolder(folderPath: string, isDirectory = false) {
		startCreation(folderPath, isDirectory);
	}


	async function handleDeleteItem(item: FileNode) {
		try {
			await $deleteFileMutation.mutateAsync(item.path);
		} catch (error) {
			console.error('Failed to delete item:', error);
			// Could add user-visible error notification here
		}
	}

	function handleRenameStart(item: FileNode) {
		// Only allow rename for non-placeholder items
		if (!item.id.startsWith('placeholder_')) {
			renamingItem = item.path;
			renameValue = item.name;
		}
	}

	async function handleRenameConfirm(item: FileNode) {
		// Handle creation confirmation
		if (creatingItem && creatingItem.path === item.path) {
			await confirmCreation();
			return;
		}

		// Handle regular rename
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
				// Reset rename state on error so user can try again
				renamingItem = null;
				renameValue = '';
				return;
			}
		}
		renamingItem = null;
		renameValue = '';
	}

	function handleRenameCancel() {
		// Handle creation cancellation
		if (creatingItem) {
			cancelCreation();
			return;
		}

		// Handle regular rename cancellation
		renamingItem = null;
		renameValue = '';
	}

	// Lifecycle management for file watcher - temporarily disabled for testing
	onMount(() => {
		// Start file watcher after component is mounted
		// fileWatcher.start(); // Temporarily disabled
	});

	onDestroy(() => {
		// Clean up file watcher on component destroy
		fileWatcher.destroy();
	});
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
				{#if !node.id.startsWith('placeholder_')}
					<ContextMenu.Separator />
					<ContextMenu.Item onclick={() => handleRenameStart(node)}>
						<Edit size={16} />
						Rename
					</ContextMenu.Item>
					<ContextMenu.Item variant="destructive" onclick={() => handleDeleteItem(node)}>
						<Trash2 size={16} />
						Delete
					</ContextMenu.Item>
				{/if}
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
				{#if !node.id.startsWith('placeholder_')}
					<ContextMenu.Item onclick={() => handleRenameStart(node)}>
						<Edit size={16} />
						Rename
					</ContextMenu.Item>
					<ContextMenu.Item variant="destructive" onclick={() => handleDeleteItem(node)}>
						<Trash2 size={16} />
						Delete
					</ContextMenu.Item>
				{/if}
			</ContextMenu.Content>
		</ContextMenu.Root>
	{/if}
{/snippet}

<div class="h-full flex flex-col">
	<div class="px-3 py-1 border-b flex items-center justify-between">
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