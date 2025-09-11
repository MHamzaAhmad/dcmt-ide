// Desktop Platform Adapters
import { DesktopFileSystemAdapter } from './filesystem';
import { DesktopProjectAdapter } from './project';
import { DesktopFileWatcher } from './fileWatcher';
import { DesktopLatexAdapter } from './latex';
import { DesktopAgentSSEAdapter } from './agentSSE';
import type { PlatformAPI } from '../../types';

// Combined Desktop API Adapter using composition with spread
export class DesktopApiAdapter implements PlatformAPI {
	private fileSystem = new DesktopFileSystemAdapter();
	private project = new DesktopProjectAdapter();
	private watcher = new DesktopFileWatcher();
	private latex = new DesktopLatexAdapter();
	private agentSSE = new DesktopAgentSSEAdapter();

	// Spread filesystem operations
	getDirectoryTree = this.fileSystem.getDirectoryTree.bind(this.fileSystem);
	readFileContent = this.fileSystem.readFileContent.bind(this.fileSystem);
	readFileRaw = this.fileSystem.readFileRaw.bind(this.fileSystem);
	writeFileContent = this.fileSystem.writeFileContent.bind(this.fileSystem);
	createFile = this.fileSystem.createFile.bind(this.fileSystem);
	deleteFile = this.fileSystem.deleteFile.bind(this.fileSystem);
	renameFile = this.fileSystem.renameFile.bind(this.fileSystem);
	fileExists = this.fileSystem.fileExists.bind(this.fileSystem);

	// Spread project operations
	selectProjectFolder = this.project.selectProjectFolder.bind(this.project);
	getCurrentProject = this.project.getCurrentProject.bind(this.project);
	clearProject = this.project.clearProject.bind(this.project);

	// LaTeX operations
	compileLatex = this.latex.compileLatex.bind(this.latex);
	findMainLatexFile = this.latex.findMainLatexFile.bind(this.latex);
	
	// Compilation events
	onCompilationEvent = this.latex.onCompilationEvent?.bind(this.latex);
	setAutoCompile = this.latex.setAutoCompile?.bind(this.latex);
	setMainFile = this.latex.setMainFile?.bind(this.latex);

	// Additional desktop-specific helpers
	getProjectInfo = this.project.getProjectInfo.bind(this.project);
	
	// Watcher functionality
	getFileWatcher() {
		return this.watcher;
	}
	
	// Agent SSE functionality
	getAgentSSE() {
		return this.agentSSE;
	}
}

// Export individual adapters for direct access if needed
export { DesktopFileSystemAdapter, DesktopProjectAdapter, DesktopFileWatcher, DesktopLatexAdapter, DesktopAgentSSEAdapter };