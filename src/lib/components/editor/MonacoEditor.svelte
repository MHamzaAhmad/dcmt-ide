<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { workspaceStore, latexStore, eventStore } from '$lib/stores';
	import { editorState } from '$lib/stores/editor.js';
	import { theme } from '$lib/stores/theme.js';
	import { debounce } from '$lib/utils/debounce';
	import type * as Monaco from 'monaco-editor';

	let editorContainer: HTMLDivElement;
	let editor: Monaco.editor.IStandaloneCodeEditor;
	let monacoInstance: typeof Monaco;
	let currentTheme = $state('light');
	let activeFilePath = $state<string | null>(null);
	let currentFiles = $state<any[]>([]);
	let isInternalUpdate = false;
	let agentIsModifyingFile = $state(false);
	let lastAgentUpdateTime = $state(0);
	let hasConflict = $state(false);
	let eventUnsubscribe: (() => void) | null = null;
	
	// Get workspace state reactively
	const workspaceState = $derived($workspaceStore);
	const openFileContents = $derived(workspaceState.openFiles);
	const latexState = $derived($latexStore);
	
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

	// Check if file is a LaTeX file
	function isLatexFile(path: string): boolean {
		const ext = path.split('.').pop()?.toLowerCase();
		return ext === 'tex';
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
	const debouncedSave = debounce(async (filePath: string, content: string) => {
		if (!filePath) return;
		
		try {
			// Update content in workspace store first
			workspaceStore.updateFileContent(filePath, content);
			
			// Save the file through workspace store - this will trigger the reactive chain
			// WorkspaceStore → LaTeXStore (if .tex file) → PDFStore automatically
			await workspaceStore.saveFile(filePath);
		} catch (error) {
			console.error('Failed to save file:', error);
		}
	}, 800);
	
	// Handle active file changes and file content updates
	$effect(() => {
		const newActiveFilePath = workspaceState.activeFile;
		const newOpenFiles = openFileContents;
		
		// Only update if activeFilePath actually changed to prevent loops
		if (newActiveFilePath !== activeFilePath) {
			activeFilePath = newActiveFilePath;
			currentFiles = newOpenFiles;
			
			if (editor && activeFilePath) {
				const activeFile = workspaceState.files.get(activeFilePath);
				if (activeFile) {
					updateEditorContent(activeFile.content || '', 'file_switch');
					
					// Update language based on file extension
					const model = editor.getModel();
					if (model && monacoInstance) {
						const language = getLanguageFromPath(activeFile.path);
						monacoInstance.editor.setModelLanguage(model, language);
					}
				}
			}
		} else {
			// File path same, but check if content changed (for agent updates)
			if (activeFilePath && editor) {
				const activeFile = workspaceState.files.get(activeFilePath);
				if (activeFile) {
					const currentEditorContent = editor.getValue();
					const fileContent = activeFile.content || '';
					
					// Check if file content differs from editor (indicates external change)
					if (fileContent !== currentEditorContent && !isInternalUpdate) {
						// Check if user has unsaved changes
						const userHasChanges = currentEditorContent !== activeFile.originalContent;
						const agentHasChanges = fileContent !== activeFile.originalContent;
						
						if (userHasChanges && agentHasChanges) {
							// Conflict: both user and agent modified
							hasConflict = true;
							console.warn('Content conflict detected between user and agent changes');
						} else if (agentHasChanges && !userHasChanges) {
							// Agent change only, safe to update
							updateEditorContent(fileContent, 'agent_update');
							lastAgentUpdateTime = Date.now();
						}
					}
				}
			}
			// Just update the files reference without changing editor content
			currentFiles = newOpenFiles;
		}
	});

	// Helper function to update editor content with proper internal update tracking
	function updateEditorContent(content: string, source: 'file_switch' | 'agent_update' | 'reload') {
		if (!editor) return;
		
		isInternalUpdate = true;
		
		if (source === 'agent_update') {
			agentIsModifyingFile = true;
			// Clear conflict state when agent updates
			hasConflict = false;
			
			// Show brief indication that agent modified the file
			setTimeout(() => {
				agentIsModifyingFile = false;
			}, 2000);
		}
		
		editor.setValue(content);
		
		setTimeout(() => { 
			isInternalUpdate = false; 
		}, 0);
	}

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
				if (activeFilePath && !isInternalUpdate) {
					const content = editor.getValue();
					workspaceStore.updateFileContent(activeFilePath, content);
					
					// Trigger debounced save
					debouncedSave(activeFilePath, content);
				}
			});
			
			// Handle manual save (Cmd/Ctrl+S)
			editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
				if (activeFilePath) {
					debouncedSave.flush();
				}
			});

			// Set initial content if there's an active file
			if (activeFilePath) {
				const activeFile = workspaceState.files.get(activeFilePath);
				if (activeFile) {
					updateEditorContent(activeFile.content || '', 'file_switch');
				}
			}
			
			// Subscribe to EventStore file system events for real-time updates
			subscribeToFileEvents();
		}
	});

	// Subscribe to EventStore file system events
	function subscribeToFileEvents() {
		if (eventUnsubscribe) {
			eventUnsubscribe();
		}
		
		// Create event stream for file system events
		const fileSystemEvents = eventStore.fileSystemEvents;
		
		eventUnsubscribe = fileSystemEvents.subscribe((events: any[]) => {
			const latestEvent = events[events.length - 1];
			if (!latestEvent || !activeFilePath) return;
			
			// Only react to events on the currently active file
			if (latestEvent.payload.path === activeFilePath) {
				console.log(`MonacoEditor: File event ${latestEvent.subtype} on active file ${activeFilePath} from ${latestEvent.payload.source}`);
				
				// Only reload for external changes (agent or file watcher), not user changes
				if (latestEvent.payload.source === 'agent' || latestEvent.payload.source === 'watcher') {
					if (latestEvent.subtype === 'file_modified') {
						// Force reload the file content from WorkspaceStore
						workspaceStore.loadFile(activeFilePath, true).then(fileContent => {
							if (fileContent && editor) {
								updateEditorContent(fileContent.content, 'agent_update');
							}
						}).catch(error => {
							console.error('Failed to reload file after agent update:', error);
						});
					}
				}
			}
		});
		
		console.log('MonacoEditor: Subscribed to file system events');
	}
	
	// Function to manually refresh file content (for conflict resolution)
	function refreshFileContent() {
		if (!activeFilePath) return;
		
		workspaceStore.loadFile(activeFilePath, true).then(fileContent => {
			if (fileContent && editor) {
				updateEditorContent(fileContent.content, 'reload');
				hasConflict = false;
			}
		}).catch(error => {
			console.error('Failed to refresh file content:', error);
		});
	}

	onDestroy(() => {
		if (editor) {
			editor.dispose();
		}
		// Cancel any pending saves
		debouncedSave.cancel();
		
		// Unsubscribe from events
		if (eventUnsubscribe) {
			eventUnsubscribe();
			eventUnsubscribe = null;
		}
	});
</script>

<div 
	bind:this={editorContainer} 
	class="h-full"
	style="min-height: 0;"
></div>