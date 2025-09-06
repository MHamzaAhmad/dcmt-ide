<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { openFiles } from '$lib/stores/files.js';
	import { editorState } from '$lib/stores/editor.js';
	import { theme } from '$lib/stores/theme.js';
	import { useWriteFileContent } from '$lib/api/hooks';
	import { debounce } from '$lib/utils/debounce';
	import type * as Monaco from 'monaco-editor';

	let editorContainer: HTMLDivElement;
	let editor: Monaco.editor.IStandaloneCodeEditor;
	let monacoInstance: typeof Monaco;
	let currentTheme = $state('light');
	let activeFileId = $state<string | null>(null);
	let currentFiles = $state<any[]>([]);
	let isInternalUpdate = false;
	
	// File save mutation
	const writeFileMutation = useWriteFileContent();
	
	// Detect language from file extension
	function getLanguageFromPath(path: string): string {
		const ext = path.split('.').pop()?.toLowerCase();
		const languageMap: Record<string, string> = {
			'js': 'javascript',
			'jsx': 'javascript',
			'ts': 'typescript',
			'tsx': 'typescript',
			'json': 'json',
			'html': 'html',
			'css': 'css',
			'scss': 'scss',
			'md': 'markdown',
			'py': 'python',
			'rs': 'rust',
			'go': 'go',
			'java': 'java',
			'cpp': 'cpp',
			'c': 'c',
			'tex': 'latex',
			'yml': 'yaml',
			'yaml': 'yaml',
			'xml': 'xml',
			'sh': 'shell',
			'bash': 'shell',
			'svelte': 'html'
		};
		return languageMap[ext || ''] || 'plaintext';
	}

	$effect(() => {
		currentTheme = $theme;
		if (editor) {
			editor.updateOptions({
				theme: currentTheme === 'dark' ? 'vs-dark' : 'vs'
			});
		}
	});

	// Create debounced save function
	const debouncedSave = debounce(async (fileId: string, content: string) => {
		const file = currentFiles.find(f => f.id === fileId);
		if (!file || !file.path || !file.isDirty) return;
		
		// Set saving status
		openFiles.setSaveStatus(fileId, 'saving');
		
		try {
			await $writeFileMutation.mutateAsync({
				path: file.path,
				content: content
			});
			
			// Mark as saved
			openFiles.markFileSaved(fileId);
		} catch (error) {
			console.error('Failed to save file:', error);
			openFiles.setSaveStatus(fileId, 'error');
		}
	}, 800);
	
	$effect(() => {
		const newActiveFileId = $editorState.activeFileId;
		const newCurrentFiles = $openFiles;
		
		// Only update if activeFileId actually changed to prevent loops
		if (newActiveFileId !== activeFileId) {
			activeFileId = newActiveFileId;
			currentFiles = newCurrentFiles;
			
			if (editor && activeFileId) {
				const activeFile = currentFiles.find(f => f.id === activeFileId);
				if (activeFile) {
					isInternalUpdate = true;
					editor.setValue(activeFile.content || '');
					
					// Update language based on file extension
					const model = editor.getModel();
					if (model && monacoInstance) {
						const language = getLanguageFromPath(activeFile.path);
						monacoInstance.editor.setModelLanguage(model, language);
					}
					
					setTimeout(() => { isInternalUpdate = false; }, 0);
				}
			}
		} else {
			// Just update the files reference without changing editor content
			currentFiles = newCurrentFiles;
		}
	});

	onMount(async () => {
		if (typeof window !== 'undefined') {
			// Configure Monaco Environment for web workers - fallback mode for dev
			self.MonacoEnvironment = {
				getWorker: function () {
					return new Worker(
						URL.createObjectURL(
							new Blob([''], { type: 'application/javascript' })
						)
					);
				}
			};
			
			// Import Monaco Editor dynamically
			const monaco = await import('monaco-editor');
			monacoInstance = monaco;

			// Configure Monaco for LaTeX
			monaco.languages.register({ id: 'latex' });

			// Define LaTeX language configuration
			monaco.languages.setLanguageConfiguration('latex', {
				comments: {
					lineComment: '%',
				},
				brackets: [
					['{', '}'],
					['[', ']'],
					['(', ')']
				],
				autoClosingPairs: [
					{ open: '{', close: '}' },
					{ open: '[', close: ']' },
					{ open: '(', close: ')' },
					{ open: '$', close: '$' },
					{ open: '$$', close: '$$' }
				],
				surroundingPairs: [
					{ open: '{', close: '}' },
					{ open: '[', close: ']' },
					{ open: '(', close: ')' },
					{ open: '$', close: '$' }
				]
			});

			// Define LaTeX syntax highlighting
			monaco.languages.setMonarchTokensProvider('latex', {
				tokenizer: {
					root: [
						[/%.*$/, 'comment'],
						[/\\[a-zA-Z@]+\*?/, 'keyword'],
						[/\\[^a-zA-Z@]/, 'keyword'],
						[/\$\$/, 'string', '@math_double'],
						[/\$/, 'string', '@math_single'],
						[/\{/, 'delimiter.curly'],
						[/\}/, 'delimiter.curly'],
						[/\[/, 'delimiter.square'],
						[/\]/, 'delimiter.square'],
						[/\(/, 'delimiter.parenthesis'],
						[/\)/, 'delimiter.parenthesis']
					],
					math_single: [
						[/[^$]+/, 'string.math'],
						[/\$/, 'string', '@pop']
					],
					math_double: [
						[/[^$]+/, 'string.math'],
						[/\$\$/, 'string', '@pop'],
						[/\$/, 'string.math']
					]
				}
			});

			// Create editor instance
			editor = monaco.editor.create(editorContainer, {
				value: '',
				language: 'plaintext', // Will be set based on file extension
				theme: currentTheme === 'dark' ? 'vs-dark' : 'vs',
				fontSize: 14,
				minimap: { enabled: true },
				wordWrap: 'on',
				lineNumbers: 'on',
				folding: true,
				automaticLayout: true,
				scrollBeyondLastLine: false,
				renderWhitespace: 'selection',
				tabSize: 2,
				insertSpaces: true
			});

			// Handle content changes
			editor.onDidChangeModelContent(() => {
				if (activeFileId && !isInternalUpdate) {
					const content = editor.getValue();
					openFiles.updateFileContent(activeFileId, content);
					
					// Trigger debounced save
					debouncedSave(activeFileId, content);
				}
			});
			
			// Handle manual save (Cmd/Ctrl+S)
			editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
				if (activeFileId) {
					debouncedSave.flush();
				}
			});

			// Set initial content if there's an active file
			if (activeFileId) {
				const activeFile = currentFiles.find(f => f.id === activeFileId);
				if (activeFile) {
					editor.setValue(activeFile.content);
				}
			}
		}
	});

	onDestroy(() => {
		if (editor) {
			editor.dispose();
		}
		// Cancel any pending saves
		debouncedSave.cancel();
	});
</script>

<div class="h-full flex flex-col">
	{#if activeFileId && currentFiles.length > 0}
		{@const activeFile = currentFiles.find(f => f.id === activeFileId)}
		<div class="h-8 border-b bg-muted/50 flex items-center px-3 text-sm">
			<span class="text-muted-foreground">{activeFile?.path || 'Untitled'}</span>
			{#if activeFile?.isDirty}
				<span class="ml-1 text-orange-500">•</span>
			{/if}
			{#if activeFile?.saveStatus === 'saving'}
				<span class="ml-2 text-xs text-muted-foreground">Saving...</span>
			{:else if activeFile?.saveStatus === 'error'}
				<span class="ml-2 text-xs text-destructive">Save failed</span>
			{/if}
		</div>
	{/if}
	
	<div 
		bind:this={editorContainer} 
		class="flex-1"
		style="min-height: 0;"
	></div>
</div>