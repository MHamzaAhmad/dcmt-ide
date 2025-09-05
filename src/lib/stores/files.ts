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
	isDirty: boolean;
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
		openFile: (file: Omit<OpenFile, 'isDirty'>) => {
			update(files => {
				const exists = files.find(f => f.id === file.id);
				if (exists) return files;
				return [...files, { ...file, isDirty: false }];
			});
		},
		closeFile: (fileId: string) => {
			update(files => files.filter(f => f.id !== fileId));
		},
		updateFileContent: (fileId: string, content: string) => {
			update(files => 
				files.map(f => 
					f.id === fileId 
						? { ...f, content, isDirty: f.content !== content }
						: f
				)
			);
		},
		markFileSaved: (fileId: string) => {
			update(files => 
				files.map(f => 
					f.id === fileId ? { ...f, isDirty: false } : f
				)
			);
		}
	};
}

export const fileTree = createFileStore();
export const openFiles = createOpenFilesStore();