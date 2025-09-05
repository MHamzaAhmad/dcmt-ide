<script lang="ts">
	import { TreeView, TreeViewFile, TreeViewFolder } from '$lib/components/ui/tree-view';
	import { File, Folder, FolderOpen, FileText } from '@lucide/svelte';
	import { fileTree, openFiles, type FileNode } from '$lib/stores/files.js';
	import { editorState } from '$lib/stores/editor.js';
	import { onMount } from 'svelte';

	let files = $state<FileNode[]>([]);

	$effect(() => {
		files = $fileTree;
	});

	onMount(() => {
		fileTree.loadMockFiles();
	});

	function handleFileClick(file: FileNode) {
		if (file.type === 'file') {
			// Open file in editor
			openFiles.openFile({
				id: file.id,
				name: file.name,
				path: file.path,
				content: getFileContent(file.path)
			});
			
			editorState.setActiveFile(file.id);
		}
	}

	function getFileContent(path: string): string {
		// Mock content based on file type
		if (path.endsWith('.tex')) {
			return `\\documentclass{article}
\\usepackage[utf8]{inputenc}
\\title{Sample LaTeX Document}
\\author{Author Name}
\\date{\\today}

\\begin{document}
\\maketitle

\\section{Introduction}
This is a sample LaTeX document.

\\section{Content}
Add your content here.

\\end{document}`;
		}
		return `// Content for ${path}`;
	}

	function renderFileTree(nodes: FileNode[]) {
		return nodes.map(node => ({
			...node,
			component: node.type === 'folder' ? 'folder' : 'file'
		}));
	}

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
		{#if files.length > 0}
			<TreeView class="w-full">
				{#each files as node}
					{#if node.type === 'folder'}
						<TreeViewFolder 
							name={node.name} 
							open={node.isExpanded}
							class="text-sm"
						>
							{#snippet icon({ name, open })}
								{#if open}
									<FolderOpen size={16} class="text-blue-500" />
								{:else}
									<Folder size={16} class="text-blue-500" />
								{/if}
							{/snippet}
							
							{#if node.children}
								{#each node.children as child}
									{#if child.type === 'file'}
										<TreeViewFile 
											name={child.name}
											class="text-sm hover:bg-accent"
											onclick={() => handleFileClick(child)}
										>
											{#snippet icon({ name })}
												{@const IconComponent = getFileIcon(name)}
												<IconComponent size={16} class="text-muted-foreground" />
											{/snippet}
										</TreeViewFile>
									{:else}
										<TreeViewFolder 
											name={child.name} 
											open={child.isExpanded}
											class="text-sm"
										>
											{#snippet icon({ name, open })}
												{#if open}
													<FolderOpen size={16} class="text-blue-500" />
												{:else}
													<Folder size={16} class="text-blue-500" />
												{/if}
											{/snippet}
											
											{#if child.children}
												{#each child.children as grandchild}
													<TreeViewFile 
														name={grandchild.name}
														class="text-sm hover:bg-accent"
														onclick={() => handleFileClick(grandchild)}
													>
														{#snippet icon({ name })}
															{@const IconComponent = getFileIcon(name)}
															<IconComponent size={16} class="text-muted-foreground" />
														{/snippet}
													</TreeViewFile>
												{/each}
											{/if}
										</TreeViewFolder>
									{/if}
								{/each}
							{/if}
						</TreeViewFolder>
					{:else}
						<TreeViewFile 
							name={node.name}
							class="text-sm hover:bg-accent"
							onclick={() => handleFileClick(node)}
						>
							{#snippet icon({ name })}
								{@const IconComponent = getFileIcon(name)}
								<IconComponent size={16} class="text-muted-foreground" />
							{/snippet}
						</TreeViewFile>
					{/if}
				{/each}
			</TreeView>
		{:else}
			<div class="p-4 text-center text-muted-foreground text-sm">
				No files found
			</div>
		{/if}
	</div>
</div>