import { writable } from 'svelte/store';

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

	return {
		subscribe,
		set,
		update,
		setActiveFile: (fileId: string | null) => {
			update(state => ({ ...state, activeFileId: fileId }));
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