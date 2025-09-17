<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { workspaceStore, eventStore } from '$lib/stores';
	import { theme } from '$lib/stores/theme.js';
	import { debounce } from '$lib/utils/debounce';
	import { editorModelRegistry } from '$lib/editor/modelRegistry';
	import { SaveController } from '$lib/editor/saveController';
	import type * as Monaco from 'monaco-editor';

	let editorContainer: HTMLDivElement;
	let editor: Monaco.editor.IStandaloneCodeEditor;
	let monacoInstance: typeof Monaco;
	let currentTheme = $state('light');
	let activeFilePath = $state<string | null>(null);
	let currentFiles = $state<any[]>([]);
	let isInternalUpdate = false;
	let saveController: SaveController | null = null;
	let currentModel: Monaco.editor.ITextModel | null = null;
	let sessionId = crypto?.randomUUID?.() ?? String(Math.random());
	let beforeUnloadHandler: (() => void) | null = null;
	let agentIsModifyingFile = $state(false);
	let lastAgentUpdateTime = $state(0);
	let hasConflict = $state(false);
	let eventUnsubscribe: (() => void) | null = null;
	
	// Get workspace state reactively
	const workspaceState = $derived($workspaceStore);
	const openFileContents = $derived(workspaceState.openFiles);
	
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



	// Legacy debounced save removed; SaveController handles persistence
	
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
				if (activeFile && monacoInstance) {
					attachModel(activeFile.path, activeFile.content || '');
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
			hasConflict = false;
			setTimeout(() => { agentIsModifyingFile = false; }, 2000);
		}
		const model = editor.getModel();
		if (model) {
			const viewState = editor.saveViewState();
			// Use pushEditOperations to preserve undo stack and avoid cursor jump
			if (model.getValue() !== content) {
				model.pushEditOperations([], [{ range: model.getFullModelRange(), text: content }], () => null);
			}
			if (viewState) editor.restoreViewState(viewState);
		} else {
			editor.setValue(content);
		}
		setTimeout(() => { isInternalUpdate = false; }, 0);
	}

	function attachModel(path: string, content: string) {
		if (!monacoInstance || !editor) return;
		const language = getLanguageFromPath(path);
		editorModelRegistry.init(monacoInstance);
		const previous = editor.getModel();
		const model = editorModelRegistry.getOrCreate(path, content, language);
		currentModel = model;
		if (previous !== model) {
			const viewState = editor.saveViewState();
			editor.setModel(model);
			if (viewState) editor.restoreViewState(viewState);
		}
		// Ensure language matches
		editorModelRegistry.setLanguage(path, language);
		// Update content if model is new/empty but content differs
		if (model.getValue() !== content) {
			updateEditorContent(content, 'file_switch');
		}
		// (Re)wire save controller for this path
		if (saveController) { saveController.dispose(); saveController = null; }
		saveController = new SaveController({
			path,
			model,
			saveFn: async (p) => {
				// Origin-aware: mark as user to prevent self-reload
				await workspaceStore.saveFile(p);
			},
			debounceMs: 800
		});
	}

	onMount(async () => {
		if (typeof window === 'undefined' || !editorContainer) return;
		
		try {
				beforeUnloadHandler = () => { try { saveController?.flush(); } catch (_) {} };
				window.addEventListener('beforeunload', beforeUnloadHandler);
			// Configure Monaco Environment with cdnjs workers (CORS-friendly)
			self.MonacoEnvironment = {
				getWorkerUrl: function (_moduleId: any, label: string) {
					const baseUrl = 'https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.52.2/min/vs';
					
					if (label === 'json') {
						return `${baseUrl}/language/json/json.worker.min.js`;
					}
					if (label === 'css' || label === 'scss' || label === 'less') {
						return `${baseUrl}/language/css/css.worker.min.js`;
					}
					if (label === 'html' || label === 'handlebars' || label === 'razor') {
						return `${baseUrl}/language/html/html.worker.min.js`;
					}
					if (label === 'typescript' || label === 'javascript') {
						return `${baseUrl}/language/typescript/ts.worker.min.js`;
					}
					// Default editor worker
					return `${baseUrl}/editor/editor.worker.min.js`;
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
					// Use SaveController for async, single-flight saves
					saveController?.schedule();
				}
			});
		
			// Handle manual save (Cmd/Ctrl+S)
			editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
				if (activeFilePath) {
					saveController?.flush();
				}
			});

			// Set initial content if there's an active file
			if (activeFilePath) {
				const activeFile = workspaceState.files.get(activeFilePath);
				if (activeFile) {
					attachModel(activeFile.path, activeFile.content || '');
				}
			}
			
			// Subscribe to EventStore file system events for real-time updates
			subscribeToFileEvents();
			
		} catch (error) {
			console.error('Failed to initialize Monaco Editor:', error);
			// Show fallback error message in the editor container
			if (editorContainer) {
				editorContainer.innerHTML = `
					<div class="h-full flex items-center justify-center bg-background text-muted-foreground">
						<div class="text-center space-y-2">
							<p>Failed to load code editor</p>
							<p class="text-sm">Please refresh the page to retry</p>
						</div>
					</div>
				`;
			}
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
	

	onDestroy(() => {
		if (editor) {
			editor.dispose();
		}
	// Cancel any pending saves
	saveController?.dispose();
		
		// Unsubscribe from events
		if (eventUnsubscribe) {
			eventUnsubscribe();
			eventUnsubscribe = null;
		}
		if (typeof window !== 'undefined' && beforeUnloadHandler) {
			window.removeEventListener('beforeunload', beforeUnloadHandler);
			beforeUnloadHandler = null;
		}
	});
</script>

<div 
	bind:this={editorContainer} 
	class="h-full"
	style="min-height: 0;"
></div>