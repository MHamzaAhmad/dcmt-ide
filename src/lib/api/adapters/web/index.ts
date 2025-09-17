// Web Platform Adapters
import { WebFileSystemAdapter } from './filesystem';
import { WebProjectAdapter } from './project';
import { WebFileWatcher } from './fileWebSocket';
import { WebLatexAdapter } from './latex';
import { AgentSSEAdapter } from './agentSSE';
import { webSocketManager } from './webSocketManager';
import type { PlatformAPI } from '../../types';

// Combined Web API Adapter using composition with spread
export class WebApiAdapter implements PlatformAPI {
	private fileSystem = new WebFileSystemAdapter();
	private project = new WebProjectAdapter();
	private fileWatcher = new WebFileWatcher();
	private latex = new WebLatexAdapter();
	private agentSSE = new AgentSSEAdapter();
	private compilationEventUnsubscribe: (() => void) | null = null;

	constructor() {}

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
	getLatexStatus = this.latex.getLatexStatus.bind(this.latex);
	
	// Compilation events
	onCompilationEvent = (callback: (event: any) => void) => {
		// Subscribe via WebSocketManager
		this.compilationEventUnsubscribe = webSocketManager.onCompilationEvent(callback);
		console.log('WebApiAdapter: Subscribed to compilation events via WebSocketManager');

		// Return unsubscribe function
		return () => {
			if (this.compilationEventUnsubscribe) {
				this.compilationEventUnsubscribe();
				this.compilationEventUnsubscribe = null;
			}
		};
	};
	setAutoCompile = this.latex.setAutoCompile?.bind(this.latex);
	setMainFile = this.latex.setMainFile?.bind(this.latex);

	// Web-specific functionality
	getFileWatcher = () => this.fileWatcher;
	getAgentSSE = () => this.agentSSE;
	getProjectInfo = this.project.getProjectInfo.bind(this.project);

	// Get WebSocket connection stats for debugging
	getWebSocketStats = () => webSocketManager.getStats();
}

// Export individual adapters for direct access if needed
export { WebFileSystemAdapter, WebProjectAdapter, WebFileWatcher, WebLatexAdapter, AgentSSEAdapter };