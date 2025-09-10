// Web Platform Adapters
import { WebFileSystemAdapter } from './filesystem';
import { WebProjectAdapter } from './project';
import { WebSocketAdapter, WebFileWatcher } from './fileWebSocket';
import { WebLatexAdapter } from './latex';
import { AgentSSEAdapter } from './agentSSE';
import type { PlatformAPI } from '../../types';

// Combined Web API Adapter using composition with spread
export class WebApiAdapter implements PlatformAPI {
	private fileSystem = new WebFileSystemAdapter();
	private project = new WebProjectAdapter();
	private websocket = new WebSocketAdapter();
	private fileWatcher = new WebFileWatcher();
	private latex = new WebLatexAdapter();
	private agentSSE = new AgentSSEAdapter();

	constructor() {
		// Auto-connect WebSocket for file watching
		if (typeof window !== 'undefined') {
			this.websocket.connect();
		}
	}

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

	// Web-specific functionality
	getWebSocket = () => this.websocket;
	getFileWatcher = () => this.fileWatcher;
	getAgentSSE = () => this.agentSSE;
	getProjectInfo = this.project.getProjectInfo.bind(this.project);
}

// Export individual adapters for direct access if needed
export { WebFileSystemAdapter, WebProjectAdapter, WebSocketAdapter, WebFileWatcher, WebLatexAdapter, AgentSSEAdapter };