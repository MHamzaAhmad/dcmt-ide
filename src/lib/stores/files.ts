import { writable } from 'svelte/store';

export interface FileNode {
	id: string;
	name: string;
	path: string;
	type: 'file' | 'folder';
	children?: FileNode[];
	isExpanded?: boolean;
}

export interface OpenFile {
	id: string;
	name: string;
	path: string;
	content: string;
	originalContent: string;
	isDirty: boolean;
	saveStatus: 'idle' | 'saving' | 'saved' | 'error';
	lastSavedAt?: number;
}

function createFileStore() {
	const { subscribe, set, update } = writable<FileNode[]>([]);

	return {
		subscribe,
		set,
		update,
		// Mock file structure for development
		loadMockFiles: () => {
			const mockFiles: FileNode[] = [
				{
					id: '1',
					name: 'project',
					path: '/project',
					type: 'folder',
					isExpanded: true,
					children: [
						{
							id: '2',
							name: 'main.tex',
							path: '/project/main.tex',
							type: 'file'
						},
						{
							id: '3',
							name: 'chapters',
							path: '/project/chapters',
							type: 'folder',
							isExpanded: false,
							children: [
								{
									id: '4',
									name: 'chapter1.tex',
									path: '/project/chapters/chapter1.tex',
									type: 'file'
								},
								{
									id: '5',
									name: 'chapter2.tex',
									path: '/project/chapters/chapter2.tex',
									type: 'file'
								}
							]
						},
						{
							id: '6',
							name: 'images',
							path: '/project/images',
							type: 'folder',
							isExpanded: false,
							children: []
						}
					]
				}
			];
			set(mockFiles);
		}
	};
}

function createOpenFilesStore() {
	const { subscribe, set, update } = writable<OpenFile[]>([]);

	return {
		subscribe,
		set,
		update,
		openFile: (file: Omit<OpenFile, 'isDirty' | 'saveStatus' | 'originalContent' | 'lastSavedAt'>) => {
			update(files => {
				const exists = files.find(f => f.id === file.id);
				if (exists) return files;
				return [...files, { 
					...file, 
					originalContent: file.content,
					isDirty: false,
					saveStatus: 'idle'
				}];
			});
		},
		closeFile: (fileId: string) => {
			update(files => files.filter(f => f.id !== fileId));
		},
		updateFileContent: (fileId: string, content: string) => {
			update(files => 
				files.map(f => 
					f.id === fileId 
						? { ...f, content, isDirty: content !== f.originalContent }
						: f
				)
			);
		},
		setSaveStatus: (fileId: string, status: 'idle' | 'saving' | 'saved' | 'error') => {
			update(files => 
				files.map(f => 
					f.id === fileId 
						? { 
							...f, 
							saveStatus: status,
							lastSavedAt: status === 'saved' ? Date.now() : f.lastSavedAt
						}
						: f
				)
			);
		},
		markFileSaved: (fileId: string) => {
			update(files => 
				files.map(f => 
					f.id === fileId 
						? { 
							...f, 
							originalContent: f.content,
							isDirty: false,
							saveStatus: 'saved',
							lastSavedAt: Date.now()
						}
						: f
				)
			);
		}
	};
}

export const fileTree = createFileStore();
export const openFiles = createOpenFilesStore();