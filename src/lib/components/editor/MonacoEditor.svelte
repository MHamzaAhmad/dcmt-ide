<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { openFiles } from '$lib/stores/files.js';
	import { editorState } from '$lib/stores/editor.js';
	import { theme } from '$lib/stores/theme.js';
	import type * as Monaco from 'monaco-editor';

	let editorContainer: HTMLDivElement;
	let editor: Monaco.editor.IStandaloneCodeEditor;
	let currentTheme = $state('light');
	let activeFileId = $state<string | null>(null);
	let currentFiles = $state<any[]>([]);

	$effect(() => {
		currentTheme = $theme;
		if (editor) {
			editor.updateOptions({
				theme: currentTheme === 'dark' ? 'vs-dark' : 'vs'
			});
		}
	});

	$effect(() => {
		activeFileId = $editorState.activeFileId;
		currentFiles = $openFiles;
		
		if (editor && activeFileId) {
			const activeFile = currentFiles.find(f => f.id === activeFileId);
			if (activeFile) {
				editor.setValue(activeFile.content);
			}
		}
	});

	onMount(async () => {
		if (typeof window !== 'undefined') {
			// Import Monaco Editor dynamically
			const monaco = await import('monaco-editor');

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
				language: 'latex',
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
				if (activeFileId) {
					const content = editor.getValue();
					openFiles.updateFileContent(activeFileId, content);
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
		</div>
	{/if}
	
	<div 
		bind:this={editorContainer} 
		class="flex-1"
		style="min-height: 0;"
	></div>
</div>