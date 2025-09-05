import { writable } from 'svelte/store';

export interface EditorState {
	activeFileId: string | null;
	isFileExplorerOpen: boolean;
	isVersionControlOpen: boolean;
	editorContent: string;
}

function createEditorStore() {
	const initialState: EditorState = {
		activeFileId: null,
		isFileExplorerOpen: true,
		isVersionControlOpen: false,
		editorContent: ''
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
		}
	};
}

export const editorState = createEditorStore();