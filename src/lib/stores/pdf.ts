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
import { isTauri } from '$lib/utils/platform';

export interface PDFDocument {
    path: string;
    url: string;
    pdfDoc: any; // PDF.js document
    numPages: number;
    loadedAt: number;
    fileSize?: number;
}

// PDFViewerState moved to component level - UI state is managed by components

export interface PDFState {
    // Core state
    isReady: boolean;

    // PDF document
    currentPdf: PDFDocument | null;
    isLoading: boolean;
    loadingProgress: number;

    // Auto-refresh
    autoRefresh: boolean;
    lastRefreshTime: number;

    // Operation-based cache invalidation
    lastOperationId: string | null;
    operationHistory: Array<{
        id: string;
        type: 'compilation' | 'agent' | 'manual' | 'file_watcher';
        source: string;
        timestamp: number;
        pdfPath?: string;
    }>;
    maxOperationHistory: number;

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
        autoRefresh: true,
        lastRefreshTime: 0,
        lastOperationId: null,
        operationHistory: [],
        maxOperationHistory: 50,
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

    // canRender removed - components manage their own rendering capability

    const currentPageInfo = derived([{ subscribe }], ([$pdf]) => {
        if (!$pdf.currentPdf) {
            return { current: 0, total: 0, text: '- / -' };
        }
        return {
            current: 1, // Default to first page
            total: $pdf.currentPdf.numPages,
            text: `1 / ${$pdf.currentPdf.numPages}`
        };
    });

    // Internal state
    let isInitialized = false;
    let latexUnsubscribe: (() => void) | null = null;
    let eventUnsubscribe: (() => void) | null = null;
    let pdfjsLib: any = null;
    let currentLoadingPath: string | null = null;
    let processedCompilationEventIds = new Set<string>(); // Track processed compilation events

    const store = {
        subscribe,
        
        // Derived stores
        hasValidPdf: { subscribe: hasValidPdf.subscribe },
        currentPageInfo: { subscribe: currentPageInfo.subscribe },

        // Initialization
        async initialize(): Promise<void> {
            if (!browser || isInitialized) return;

            console.log('PDFStore: Initializing...');

            try {
                // Load PDF.js library
                await store.loadPDFJS();
                
                // Subscribe to EventStore compilation events (unified approach)
                store.subscribeToCompilationEvents();

                // No legacy event listeners - using pure EventStore pattern

                // Check for existing PDFs
                const latexState = latexStore.getCurrentState();
                console.log(`PDFStore: Current LaTeX state - mainFile: ${latexState.mainFile}, currentPdfPath: ${latexState.currentPdfPath}`);
                if (latexState.currentPdfPath) {
                    const initOperationId = store.createOperationId('manual', 'initialization');
                    await store.loadPdf(latexState.currentPdfPath, initOperationId, 'manual', 'initialization');
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
                
                // Enable graceful degradation immediately for worker loading failures
                // Don't retry endlessly - just enable degraded mode
                console.log('PDFStore: Enabling graceful degradation due to initialization failure');
                store.enableGracefulDegradation();
                
                // Mark as initialized but not ready (degraded mode)
                isInitialized = true;
            }
        },

        // PDF.js loading
        async loadPDFJS(): Promise<void> {
            try {
                pdfjsLib = await import('pdfjs-dist');
                
                // Use jsdelivr CDN which has proper CORS headers for cross-origin requests
                // For PDF.js 5.x+, use .mjs extension as per jsdelivr package structure
                if (browser) {
                    pdfjsLib.GlobalWorkerOptions.workerSrc = 
                        `https://cdn.jsdelivr.net/npm/pdfjs-dist@${pdfjsLib.version}/build/pdf.worker.min.mjs`;
                }

                console.log('PDFStore: PDF.js loaded successfully with CDN worker');
                
            } catch (error) {
                console.error('PDFStore: Failed to load PDF.js:', error);
                
                // Emit system error event following EventStore pattern
                if (eventStore) {
                    eventStore.events.systemError('PDF.js initialization failed - PDF preview unavailable');
                }
                
                throw new Error('Failed to load PDF viewer library');
            }
        },

        // Graceful degradation when PDF system fails
        enableGracefulDegradation(): void {
            console.log('PDFStore: Enabling graceful degradation mode');
            
            update(state => ({
                ...state,
                isReady: false,
                error: 'PDF preview unavailable - application remains functional for LaTeX editing',
                currentPdf: null
            }));

            // Emit event following EventStore pattern
            if (browser && eventStore) {
                eventStore.events.systemError('PDF preview disabled - LaTeX editing still available');
            }

            // Still subscribe to compilation events in case PDF system recovers later
            if (browser && eventStore) {
                const compilationEvents = eventStore.compilationEvents;
                compilationEvents.subscribe(events => {
                    const latestEvent = events[events.length - 1];
                    if (latestEvent && latestEvent.subtype === 'completed') {
                        console.log('PDFStore: LaTeX compiled but PDF preview unavailable');
                    }
                });
            }
        },

        // Legacy event handlers removed - using EventStore reactive streams

        // Operation tracking helpers
        createOperationId(type: 'compilation' | 'agent' | 'manual' | 'file_watcher', source: string): string {
            return `${type}-${source}-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`;
        },

        addOperation(type: 'compilation' | 'agent' | 'manual' | 'file_watcher', source: string, operationId: string, pdfPath?: string): void {
            update(state => {
                const newOperation = {
                    id: operationId,
                    type,
                    source,
                    timestamp: Date.now(),
                    pdfPath
                };
                
                const newHistory = [newOperation, ...state.operationHistory]
                    .slice(0, state.maxOperationHistory);
                
                return {
                    ...state,
                    operationHistory: newHistory,
                    lastOperationId: operationId
                };
            });
            console.log(`PDFStore: Added operation ${operationId} (${type}/${source}) for PDF: ${pdfPath || 'unknown'}`);
        },

        shouldSkipOperation(operationId: string, pdfPath: string): boolean {
            const currentState = get({ subscribe });
            
            // Check if this exact operation was already processed
            const existingOperation = currentState.operationHistory.find(op => op.id === operationId);
            if (existingOperation) {
                console.log(`PDFStore: Skipping duplicate operation ${operationId} for PDF: ${pdfPath}`);
                return true;
            }
            
            // Check for recent operations on the same PDF within a short time window (500ms)
            const recentOperations = currentState.operationHistory.filter(op => 
                op.pdfPath === pdfPath && 
                (Date.now() - op.timestamp) < 500
            );
            
            if (recentOperations.length > 0) {
                console.log(`PDFStore: Skipping operation ${operationId} - recent operation found for PDF: ${pdfPath}`);
                return true;
            }
            
            return false;
        },

        // PDF loading
        async loadPdf(pdfPath: string, operationId?: string, operationType: 'compilation' | 'agent' | 'manual' | 'file_watcher' = 'manual', operationSource: string = 'unknown'): Promise<void> {
            if (!pdfjsLib) {
                console.error('PDFStore: PDF.js not loaded');
                return;
            }

            // Check if in graceful degradation mode - don't attempt PDF loading
            let currentState = get({ subscribe });
            if (!currentState.isReady && currentState.error?.includes('PDF preview unavailable')) {
                console.log(`PDFStore: In graceful degradation mode - skipping PDF load for ${pdfPath}`);
                return;
            }

            // Generate operation ID if not provided
            const finalOperationId = operationId || store.createOperationId(operationType, operationSource);
            
            // Check if we should skip this operation (duplicate prevention)
            // But allow retries with the same operation ID
            if (operationId && store.shouldSkipOperation(operationId, pdfPath)) {
                const currentState = get({ subscribe });
                const isRetry = currentState.retryCount > 0;
                if (!isRetry) {
                    return;
                }
                // Allow retries to proceed
                console.log(`PDFStore: Allowing retry for operation ${operationId} (retry count: ${currentState.retryCount})`);
            }

            // Prevent concurrent loading of the same PDF
            if (currentLoadingPath === pdfPath) {
                console.log(`PDFStore: Already loading ${pdfPath}, skipping duplicate request (${finalOperationId})`);
                return;
            }

            currentState = get({ subscribe });
            
            // Force reload for compilation events - PDF content may have changed
            if (currentState.currentPdf?.path === pdfPath && !currentState.error) {
                console.log(`PDFStore: PDF ${pdfPath} already loaded, but forcing reload for updated content (${finalOperationId})`);
                // Continue with reload to get updated content
            }

            console.log(`PDFStore: Loading PDF: ${pdfPath} (operation: ${finalOperationId})`);
            
            // Add operation to history
            store.addOperation(operationType, operationSource, finalOperationId, pdfPath);
            
            // Mark as currently loading
            currentLoadingPath = pdfPath;

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
                
                // Add cache busting parameter to force fresh load (except for data URLs)
                const cacheBustedUrl = typeof pdfUrl === 'string' && !pdfUrl.startsWith('data:')
                    ? (pdfUrl.includes('?') 
                        ? `${pdfUrl}&_t=${Date.now()}` 
                        : `${pdfUrl}?_t=${Date.now()}`)
                    : pdfUrl; // Don't modify data URLs
                
                console.log(`PDFStore: Loading PDF with URL: ${cacheBustedUrl.substring(0, 50)}${cacheBustedUrl.length > 50 ? '...' : ''}`);
                
                // For desktop URLs, handle different types appropriately
                if (isTauri()) {
                    if (cacheBustedUrl.startsWith('data:')) {
                        console.log('PDFStore: Desktop data URL detected - this is the expected format for PDFs');
                        console.log('PDFStore: Data URL is immediately available for PDF.js');
                        // Data URLs work perfectly with PDF.js, no delay needed
                    } else if (cacheBustedUrl.startsWith('blob:')) {
                        console.log('PDFStore: Desktop blob URL detected, adding readiness delay...');
                        await new Promise(resolve => setTimeout(resolve, 100));
                    } else if (cacheBustedUrl.startsWith('asset:')) {
                        console.warn('PDFStore: Asset URL detected - this may have CORS issues with PDF.js');
                    }
                }
                
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
                        recentPdfs: newRecentPdfs
                    };
                });

                console.log(`PDFStore: PDF loaded successfully - ${pdfDoc.numPages} pages`);
                
                // Clear loading state
                currentLoadingPath = null;

                // Note: Canvas rendering is now handled by the PDFPreview component
                // through reactive effects when PDF state changes

            } catch (error) {
                console.error(`PDFStore: Failed to load PDF ${pdfPath}:`, error);
                
                // Clear loading state on error
                currentLoadingPath = null;
                
                const errorMessage = store.getErrorMessage(error);
                
                update(state => ({
                    ...state,
                    isLoading: false,
                    error: errorMessage,
                    retryCount: state.retryCount + 1
                }));

                // Enable graceful degradation instead of endless retries
                const currentRetryState = get({ subscribe });
                if (currentRetryState.retryCount < currentRetryState.maxRetries) {
                    const retryDelay = Math.pow(2, currentRetryState.retryCount) * 1000;
                    console.log(`PDFStore: Retrying in ${retryDelay}ms (attempt ${currentRetryState.retryCount + 1})`);
                    
                    setTimeout(() => {
                        // Preserve the operation context for retry - use the same operation ID to prevent infinite loops
                        store.loadPdf(pdfPath, finalOperationId, operationType, operationSource);
                    }, retryDelay);
                } else {
                    console.log('PDFStore: Max retries reached, enabling graceful degradation');
                    store.enableGracefulDegradation();
                }
            }
        },

        // Canvas management removed - components manage their own canvases

        // Rendering removed - components manage their own rendering

        // Viewer controls removed - components manage their own UI state

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
                const refreshOperationId = store.createOperationId('manual', 'user_refresh');
                await store.loadPdf(currentState.currentPdf.path, refreshOperationId, 'manual', 'user_refresh');
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
                } else if (error.message.includes('WebKitBlobResource error') || 
                          (isTauri() && error.message.includes('Unexpected server response (0)'))) {
                    return 'Desktop file access error. The PDF file may be temporarily unavailable.';
                } else if (error.message.includes('ResponseException') && isTauri()) {
                    return 'Desktop PDF loading failed. Retrying automatically...';
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

        getOperationHistory(): Array<{ id: string; type: string; source: string; timestamp: number; pdfPath?: string; }> {
            const state = get({ subscribe });
            return [...state.operationHistory];
        },

        // Subscribe to EventStore compilation events
        subscribeToCompilationEvents(): void {
            // Create event streams
            const compilationEvents = eventStore.compilationEvents;
            
            // React to compilation completion
            const compilationUnsubscribe = compilationEvents.subscribe(events => {
                // Only process new compilation events we haven't seen before
                const newCompletionEvents = events.filter(event => 
                    event.subtype === 'completed' && 
                    !processedCompilationEventIds.has(`${event.payload.timestamp}-${event.payload.pdfPath}`)
                );
                
                if (newCompletionEvents.length === 0) {
                    return; // No new completion events to process
                }
                
                const latestEvent = newCompletionEvents[newCompletionEvents.length - 1];
                // Mark this event as processed
                const eventId = `${latestEvent.payload.timestamp}-${latestEvent.payload.pdfPath}`;
                processedCompilationEventIds.add(eventId);
                
                console.log('✅ PDFStore: Processing NEW compilation completion event:', latestEvent);
                
                if (latestEvent && latestEvent.subtype === 'completed') {
                    const pdfPath = latestEvent.payload.pdfPath;
                    
                    if (pdfPath) {
                        const currentState = get({ subscribe });
                        
                        if (currentState.autoRefresh) {
                            // Create operation-specific ID for compilation events
                            const compilationOperationId = store.createOperationId(
                                'compilation', 
                                'latex'
                            );
                            
                            console.log(`PDFStore: Compilation completed, loading PDF: ${pdfPath} (operation: ${compilationOperationId})`);
                            store.loadPdf(pdfPath, compilationOperationId, 'compilation', 'latex');
                        }
                    }
                }
            });
            
            eventUnsubscribe = () => {
                compilationUnsubscribe();
            };
            console.log('PDFStore: Subscribed to compilation events');
        },


        // Cleanup
        async destroy(): Promise<void> {
            // Clean up PDF document
            const currentState = get({ subscribe });
            if (currentState.currentPdf?.pdfDoc && typeof currentState.currentPdf.pdfDoc.destroy === 'function') {
                currentState.currentPdf.pdfDoc.destroy();
            }

            // No legacy event listeners to remove

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