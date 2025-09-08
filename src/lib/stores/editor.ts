import { writable, derived } from 'svelte/store';
import { eventStore } from './events';

export interface EditorState {
	activeFileId: string | null;
	isFileExplorerOpen: boolean;
	isVersionControlOpen: boolean;
	editorContent: string;
	activeTab: 'code' | 'chat';
}

function createEditorStore() {
	const initialState: EditorState = {
		activeFileId: null,
		isFileExplorerOpen: true,
		isVersionControlOpen: false,
		editorContent: '',
		activeTab: 'code'
	};

	const { subscribe, set, update } = writable<EditorState>(initialState);
	
	// Subscribe to UI events from EventStore
	eventStore.uiEvents.subscribe(events => {
		const latestFileEvent = events
			.filter(e => e.subtype === 'file_opened' || e.subtype === 'file_closed')
			.pop();
		
		if (latestFileEvent?.subtype === 'file_opened' && latestFileEvent.payload.filePath) {
			update(state => ({
				...state,
				activeFileId: latestFileEvent.payload.filePath || null
			}));
		} else if (latestFileEvent?.subtype === 'file_closed') {
			update(state => ({
				...state,
				activeFileId: null
			}));
		}
	});

	return {
		subscribe,
		set,
		update,
		setActiveFile: (fileId: string | null) => {
			update(state => ({ ...state, activeFileId: fileId }));
			
			// Emit UI event to EventStore
			if (fileId) {
				eventStore.events.fileOpened(fileId);
			} else {
				eventStore.emit({
					type: 'ui',
					subtype: 'file_closed',
					payload: { timestamp: Date.now() }
				});
			}
		},
		toggleFileExplorer: () => {
			update(state => ({ ...state, isFileExplorerOpen: !state.isFileExplorerOpen }));
		},
		toggleVersionControl: () => {
			update(state => ({ ...state, isVersionControlOpen: !state.isVersionControlOpen }));
		},
		setEditorContent: (content: string) => {
			update(state => ({ ...state, editorContent: content }));
		},
		setActiveTab: (tab: 'code' | 'chat') => {
			update(state => ({ ...state, activeTab: tab }));
		}
	};
}

export const editorState = createEditorStore();