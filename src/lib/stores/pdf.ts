/**
 * PDFStore - Reactive PDF preview management
 * Automatically loads and refreshes PDF previews when LaTeX compilation succeeds
 */

import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import { latexStore } from './latex';
import { workspaceStore } from './workspace';
import { eventStore } from './events';
import { platformApi } from '$lib/api/adapters';

export interface PDFDocument {
    path: string;
    url: string;
    pdfDoc: any; // PDF.js document
    numPages: number;
    loadedAt: number;
    fileSize?: number;
}

export interface PDFViewerState {
    currentPage: number;
    scale: number;
    rotation: number;
    viewMode: 'fit-width' | 'fit-height' | 'fit-page' | 'actual-size';
}

export interface PDFState {
    // Core state
    isReady: boolean;
    
    // PDF document
    currentPdf: PDFDocument | null;
    isLoading: boolean;
    loadingProgress: number;
    
    // Viewer state
    viewer: PDFViewerState;
    
    // Canvas state
    canvas: HTMLCanvasElement | null;
    context: CanvasRenderingContext2D | null;
    isRendering: boolean;
    
    // Auto-refresh
    autoRefresh: boolean;
    lastRefreshTime: number;
    
    // Error handling
    error: string | null;
    retryCount: number;
    maxRetries: number;
    
    // History
    recentPdfs: string[];
    maxRecentPdfs: number;
}

function createPdfStore() {
    const initialState: PDFState = {
        isReady: false,
        currentPdf: null,
        isLoading: false,
        loadingProgress: 0,
        viewer: {
            currentPage: 1,
            scale: 1.5,
            rotation: 0,
            viewMode: 'fit-width'
        },
        canvas: null,
        context: null,
        isRendering: false,
        autoRefresh: true,
        lastRefreshTime: 0,
        error: null,
        retryCount: 0,
        maxRetries: 3,
        recentPdfs: [],
        maxRecentPdfs: 10
    };

    const { subscribe, set, update } = writable<PDFState>(initialState);

    // Derived stores
    const hasValidPdf = derived([{ subscribe }], ([$pdf]) => {
        return !!(
            $pdf.currentPdf && 
            $pdf.currentPdf.pdfDoc && 
            !$pdf.isLoading && 
            !$pdf.error
        );
    });

    const canRender = derived([{ subscribe }], ([$pdf]) => {
        return !!(
            $pdf.currentPdf?.pdfDoc && 
            $pdf.canvas && 
            $pdf.context && 
            !$pdf.isRendering
        );
    });

    const currentPageInfo = derived([{ subscribe }], ([$pdf]) => {
        if (!$pdf.currentPdf) {
            return { current: 0, total: 0, text: '- / -' };
        }
        return {
            current: $pdf.viewer.currentPage,
            total: $pdf.currentPdf.numPages,
            text: `${$pdf.viewer.currentPage} / ${$pdf.currentPdf.numPages}`
        };
    });

    // Internal state
    let isInitialized = false;
    let latexUnsubscribe: (() => void) | null = null;
    let eventUnsubscribe: (() => void) | null = null;
    let pdfjsLib: any = null;

    const store = {
        subscribe,
        
        // Derived stores  
        hasValidPdf: { subscribe: hasValidPdf.subscribe },
        canRender: { subscribe: canRender.subscribe },
        currentPageInfo: { subscribe: currentPageInfo.subscribe },

        // Initialization
        async initialize(): Promise<void> {
            if (!browser || isInitialized) return;

            console.log('PDFStore: Initializing...');

            try {
                // Load PDF.js library
                await store.loadPDFJS();
                
                // Subscribe to LaTeX compilation results
                latexUnsubscribe = latexStore.subscribe($latex => {
                    store.handleLatexStateChange($latex);
                });

                // Subscribe to EventStore compilation events for better coordination
                store.subscribeToCompilationEvents();

                // Keep legacy event listeners as fallback
                if (browser) {
                    window.addEventListener('latex-compiled', store.handleLatexCompiled);
                    window.addEventListener('latex-compile-error', store.handleLatexError);
                }

                // Check for existing PDFs
                const latexState = latexStore.getCurrentState();
                if (latexState.currentPdfPath) {
                    await store.loadPdf(latexState.currentPdfPath);
                } else {
                    // Check if workspace has a main LaTeX file with corresponding PDF
                    const workspaceState = workspaceStore.getCurrentState();
                    if (workspaceState.mainLatexFile) {
                        const potentialPdfPath = workspaceState.mainLatexFile.replace(/\.tex$/, '.pdf');
                        console.log(`PDFStore: Checking for existing PDF: ${potentialPdfPath}`);
                        
                        try {
                            // Try to load the PDF if it exists
                            await store.loadPdf(potentialPdfPath);
                        } catch (error) {
                            console.log(`PDFStore: No existing PDF found at ${potentialPdfPath}`);
                        }
                    }
                }

                update(state => ({
                    ...state,
                    isReady: true
                }));

                isInitialized = true;
                console.log('PDFStore: Initialized successfully');

            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : 'Failed to initialize PDF store';
                console.error('PDFStore initialization failed:', errorMessage);
                
                update(state => ({
                    ...state,
                    error: errorMessage
                }));
            }
        },

        // PDF.js loading
        async loadPDFJS(): Promise<void> {
            try {
                pdfjsLib = await import('pdfjs-dist');
                
                // Set worker path
                pdfjsLib.GlobalWorkerOptions.workerSrc = new URL(
                    'pdfjs-dist/build/pdf.worker.mjs',
                    import.meta.url
                ).toString();

                console.log('PDFStore: PDF.js loaded successfully');
                
            } catch (error) {
                console.error('PDFStore: Failed to load PDF.js:', error);
                throw new Error('Failed to load PDF viewer library');
            }
        },

        // LaTeX integration
        handleLatexStateChange(latexState: any): void {
            const currentState = get({ subscribe });
            
            // Auto-refresh when compilation succeeds
            if (latexState.compilationStatus === 'success' && 
                latexState.currentPdfPath && 
                currentState.autoRefresh) {
                
                console.log(`PDFStore: LaTeX compilation succeeded, loading PDF: ${latexState.currentPdfPath}`);
                store.loadPdf(latexState.currentPdfPath);
            }
        },

        handleLatexCompiled(event: Event): void {
            const customEvent = event as CustomEvent;
            const { outputFile } = customEvent.detail;
            
            if (outputFile) {
                console.log(`PDFStore: LaTeX compiled successfully, loading: ${outputFile}`);
                store.loadPdf(outputFile);
            }
        },

        handleLatexError(event: Event): void {
            const customEvent = event as CustomEvent;
            console.log('PDFStore: LaTeX compilation failed, keeping current PDF');
            
            // Don't clear the current PDF on compilation errors
            // Users can still view the last successful version
        },

        // PDF loading
        async loadPdf(pdfPath: string): Promise<void> {
            if (!pdfjsLib) {
                console.error('PDFStore: PDF.js not loaded');
                return;
            }

            const currentState = get({ subscribe });
            
            // Force reload for compilation events - PDF content may have changed
            if (currentState.currentPdf?.path === pdfPath && !currentState.error) {
                console.log(`PDFStore: PDF ${pdfPath} already loaded, but forcing reload for updated content`);
                // Continue with reload to get updated content
            }

            console.log(`PDFStore: Loading PDF: ${pdfPath}`);

            update(state => ({
                ...state,
                isLoading: true,
                loadingProgress: 0,
                error: null,
                retryCount: 0
            }));

            try {
                // Get PDF URL from platform API with cache busting
                const pdfUrl = await platformApi.readFileRaw(pdfPath);
                
                // Add cache busting parameter to force fresh load
                const cacheBustedUrl = typeof pdfUrl === 'string' && pdfUrl.includes('?') 
                    ? `${pdfUrl}&_t=${Date.now()}` 
                    : typeof pdfUrl === 'string' 
                        ? `${pdfUrl}?_t=${Date.now()}` 
                        : pdfUrl;
                
                console.log(`PDFStore: Loading PDF with cache busting: ${cacheBustedUrl}`);
                
                // Load PDF document
                const loadingTask = pdfjsLib.getDocument(cacheBustedUrl);
                
                // Progress tracking
                loadingTask.onProgress = (progressData: any) => {
                    if (progressData.total > 0) {
                        const progress = (progressData.loaded / progressData.total) * 100;
                        update(state => ({
                            ...state,
                            loadingProgress: Math.round(progress)
                        }));
                    }
                };

                const pdfDoc = await loadingTask.promise;
                
                const pdfDocument: PDFDocument = {
                    path: pdfPath,
                    url: pdfUrl,
                    pdfDoc,
                    numPages: pdfDoc.numPages,
                    loadedAt: Date.now()
                };

                update(state => {
                    const newRecentPdfs = [pdfPath, ...state.recentPdfs.filter(p => p !== pdfPath)]
                        .slice(0, state.maxRecentPdfs);
                    
                    return {
                        ...state,
                        currentPdf: pdfDocument,
                        isLoading: false,
                        loadingProgress: 100,
                        error: null,
                        lastRefreshTime: Date.now(),
                        recentPdfs: newRecentPdfs,
                        viewer: {
                            ...state.viewer,
                            currentPage: 1 // Reset to first page
                        }
                    };
                });

                console.log(`PDFStore: PDF loaded successfully - ${pdfDoc.numPages} pages`);
                
                // Render first page if canvas is ready
                const updatedState = get({ subscribe });
                console.log(`PDFStore: Checking canvas availability - canvas: ${!!updatedState.canvas}, context: ${!!updatedState.context}`);
                if (updatedState.canvas && updatedState.context) {
                    console.log(`PDFStore: Canvas ready, rendering first page...`);
                    await store.renderCurrentPage();
                } else {
                    console.log(`PDFStore: Canvas not ready yet, will render when canvas becomes available`);
                }

            } catch (error) {
                console.error(`PDFStore: Failed to load PDF ${pdfPath}:`, error);
                
                const errorMessage = store.getErrorMessage(error);
                
                update(state => ({
                    ...state,
                    isLoading: false,
                    error: errorMessage,
                    retryCount: state.retryCount + 1
                }));

                // Auto-retry with exponential backoff
                const currentRetryState = get({ subscribe });
                if (currentRetryState.retryCount < currentRetryState.maxRetries) {
                    const retryDelay = Math.pow(2, currentRetryState.retryCount) * 1000;
                    console.log(`PDFStore: Retrying in ${retryDelay}ms (attempt ${currentRetryState.retryCount + 1})`);
                    
                    setTimeout(() => {
                        store.loadPdf(pdfPath);
                    }, retryDelay);
                }
            }
        },

        // Canvas management
        setCanvas(canvas: HTMLCanvasElement | null): void {
            const context = canvas?.getContext('2d') || null;
            console.log(`PDFStore: Setting canvas - canvas: ${!!canvas}, context: ${!!context}`);
            
            update(state => ({
                ...state,
                canvas,
                context
            }));

            // Render current page if PDF is loaded
            if (canvas && context) {
                const currentState = get({ subscribe });
                console.log(`PDFStore: Canvas set and PDF available: ${!!currentState.currentPdf}`);
                if (currentState.currentPdf) {
                    console.log(`PDFStore: Rendering PDF on canvas set`);
                    store.renderCurrentPage();
                }
            }
        },

        // Rendering
        async renderCurrentPage(): Promise<void> {
            const currentState = get({ subscribe });
            
            if (!currentState.currentPdf?.pdfDoc || !currentState.canvas || !currentState.context) {
                console.log('PDFStore: Cannot render - missing PDF document or canvas');
                return;
            }

            if (currentState.isRendering) {
                console.log('PDFStore: Already rendering, skipping');
                return;
            }

            update(state => ({ ...state, isRendering: true }));

            try {
                const page = await currentState.currentPdf.pdfDoc.getPage(currentState.viewer.currentPage);
                const viewport = page.getViewport({ 
                    scale: currentState.viewer.scale,
                    rotation: currentState.viewer.rotation 
                });

                // Update canvas size
                currentState.canvas.width = viewport.width;
                currentState.canvas.height = viewport.height;

                // Clear canvas
                currentState.context.clearRect(0, 0, viewport.width, viewport.height);

                // Render page
                const renderContext = {
                    canvasContext: currentState.context,
                    viewport: viewport
                };

                await page.render(renderContext).promise;
                
                console.log(`PDFStore: Page ${currentState.viewer.currentPage} rendered successfully`);

            } catch (error) {
                console.error('PDFStore: Error rendering page:', error);
                update(state => ({
                    ...state,
                    error: 'Failed to render PDF page'
                }));
            } finally {
                update(state => ({ ...state, isRendering: false }));
            }
        },

        // Viewer controls
        nextPage(): void {
            const currentState = get({ subscribe });
            if (currentState.currentPdf && currentState.viewer.currentPage < currentState.currentPdf.numPages) {
                update(state => ({
                    ...state,
                    viewer: { ...state.viewer, currentPage: state.viewer.currentPage + 1 }
                }));
                store.renderCurrentPage();
            }
        },

        prevPage(): void {
            const currentState = get({ subscribe });
            if (currentState.viewer.currentPage > 1) {
                update(state => ({
                    ...state,
                    viewer: { ...state.viewer, currentPage: state.viewer.currentPage - 1 }
                }));
                store.renderCurrentPage();
            }
        },

        goToPage(pageNumber: number): void {
            const currentState = get({ subscribe });
            if (currentState.currentPdf && pageNumber >= 1 && pageNumber <= currentState.currentPdf.numPages) {
                update(state => ({
                    ...state,
                    viewer: { ...state.viewer, currentPage: pageNumber }
                }));
                store.renderCurrentPage();
            }
        },

        zoomIn(): void {
            update(state => ({
                ...state,
                viewer: { ...state.viewer, scale: Math.min(state.viewer.scale + 0.2, 3.0) }
            }));
            store.renderCurrentPage();
        },

        zoomOut(): void {
            update(state => ({
                ...state,
                viewer: { ...state.viewer, scale: Math.max(state.viewer.scale - 0.2, 0.5) }
            }));
            store.renderCurrentPage();
        },

        setScale(scale: number): void {
            update(state => ({
                ...state,
                viewer: { ...state.viewer, scale: Math.max(0.5, Math.min(scale, 3.0)) }
            }));
            store.renderCurrentPage();
        },

        rotate(): void {
            update(state => ({
                ...state,
                viewer: { 
                    ...state.viewer, 
                    rotation: (state.viewer.rotation + 90) % 360 
                }
            }));
            store.renderCurrentPage();
        },

        // Settings
        setAutoRefresh(enabled: boolean): void {
            update(state => ({
                ...state,
                autoRefresh: enabled
            }));
            console.log(`PDFStore: Auto-refresh ${enabled ? 'enabled' : 'disabled'}`);
        },

        // Manual refresh
        async refresh(): Promise<void> {
            const currentState = get({ subscribe });
            if (currentState.currentPdf) {
                await store.loadPdf(currentState.currentPdf.path);
            }
        },

        // Error handling
        getErrorMessage(error: any): string {
            if (error instanceof Error) {
                if (error.message.includes('404') || error.message.includes('Not Found')) {
                    return 'PDF file not found. Compilation may have failed.';
                } else if (error.message.includes('network')) {
                    return 'Network error loading PDF. Please check your connection.';
                } else if (error.message.includes('InvalidPDFException')) {
                    return 'Invalid PDF file. The compilation may have produced a corrupted file.';
                } else {
                    return `Failed to load PDF: ${error.message}`;
                }
            }
            return 'Unknown error loading PDF document';
        },

        clearError(): void {
            update(state => ({
                ...state,
                error: null,
                retryCount: 0
            }));
        },

        // Utilities
        getCurrentState(): PDFState {
            return get({ subscribe });
        },

        hasPdf(): boolean {
            const state = get({ subscribe });
            return !!state.currentPdf;
        },

        getPdfPath(): string | null {
            const state = get({ subscribe });
            return state.currentPdf?.path || null;
        },

        // Subscribe to EventStore compilation events
        subscribeToCompilationEvents(): void {
            // Create event stream for compilation events
            const compilationEvents = eventStore.compilationEvents;
            
            // React to compilation completion
            const unsubscribe = compilationEvents.subscribe(events => {
                const latestEvent = events[events.length - 1];
                if (latestEvent && latestEvent.subtype === 'completed') {
                    const pdfPath = latestEvent.payload.pdfPath;
                    
                    if (pdfPath) {
                        const currentState = get({ subscribe });
                        if (currentState.autoRefresh) {
                            console.log(`PDFStore: Compilation completed, force loading PDF: ${pdfPath}`);
                            store.loadPdf(pdfPath); // Will now force reload due to cache busting
                        }
                    }
                }
            });
            
            eventUnsubscribe = unsubscribe;
            console.log('PDFStore: Subscribed to compilation events');
        },

        // Agent integration - handle external PDF updates
        handleAgentFileOperation(tool: string, path: string): void {
            if (path.endsWith('.pdf')) {
                console.log(`PDFStore: Agent ${tool} on PDF file ${path}`);
                
                const currentState = get({ subscribe });
                if (currentState.autoRefresh && (tool === 'write_file' || tool === 'create_file')) {
                    store.loadPdf(path);
                }
            }
        },

        // Cleanup
        async destroy(): Promise<void> {
            // Clean up PDF document
            const currentState = get({ subscribe });
            if (currentState.currentPdf?.pdfDoc && typeof currentState.currentPdf.pdfDoc.destroy === 'function') {
                currentState.currentPdf.pdfDoc.destroy();
            }

            // Remove event listeners
            if (browser) {
                window.removeEventListener('latex-compiled', store.handleLatexCompiled);
                window.removeEventListener('latex-compile-error', store.handleLatexError);
            }

            // Unsubscribe from EventStore events
            if (eventUnsubscribe) {
                eventUnsubscribe();
                eventUnsubscribe = null;
            }
            
            // Unsubscribe from LaTeX store
            if (latexUnsubscribe) {
                latexUnsubscribe();
                latexUnsubscribe = null;
            }

            // Reset state
            set(initialState);
            isInitialized = false;
            
            console.log('PDFStore: Destroyed and cleaned up');
        }
    };

    return store;
}

export const pdfStore = createPdfStore();