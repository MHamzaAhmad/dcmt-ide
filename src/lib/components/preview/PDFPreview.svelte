<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { ZoomIn, ZoomOut, RotateCw, Download, FileText } from '@lucide/svelte';

	// Props
	interface Props {
		showToolbar?: boolean;
		fullPanel?: boolean;
	}
	
	let { showToolbar = true, fullPanel = false }: Props = $props();

	let canvasContainer: HTMLDivElement;
	let canvas = $state<HTMLCanvasElement>();
	let pdfDoc = $state<any>(null);
	let currentPage = $state(1);
	let totalPages = $state(0);
	let scale = $state(1.5);
	let rotation = $state(0);
	let isLoading = $state(false);
	let error = $state<string | null>(null);
	let compilationErrors = $state<string[]>([]);
	let isCompiling = $state(false);

	const mockPdfPath = '/sample.pdf'; // This would be generated from LaTeX compilation

	// Convert relative path from LaTeX compilation to accessible URL
	function convertToAccessiblePath(relativePath: string): string {
		// For desktop (Tauri), we'll need to use the Tauri API to read files
		// For web, we'll need to serve files through the backend
		// For now, we'll construct a URL that can be served by the backend
		const baseUrl = import.meta.env.VITE_API_BASE_URL || 'http://localhost:3001';
		
		// Remove leading slash if present and ensure proper path formatting
		const cleanPath = relativePath.startsWith('/') ? relativePath.slice(1) : relativePath;
		
		// Construct the URL for accessing the PDF file
		return `${baseUrl}/files/workspace/${cleanPath}`;
	}

	onMount(async () => {
		await loadPDFJS();
		// Load a sample PDF or show placeholder
		showPlaceholder();
		
		// Listen for LaTeX compilation events
		const handleLatexCompiled = (event: CustomEvent) => {
			const { outputFile, message } = event.detail;
			isCompiling = false;
			compilationErrors = [];
			error = null;
			
			if (outputFile) {
				console.log('LaTeX compiled successfully:', message);
				// Convert relative path to accessible URL
				const pdfUrl = convertToAccessiblePath(outputFile);
				loadPDF(pdfUrl);
			}
		};
		
		const handleLatexError = (event: CustomEvent) => {
			const { errors, message } = event.detail;
			console.error('LaTeX compilation failed:', message, errors);
			
			isCompiling = false;
			compilationErrors = errors || [];
			error = `LaTeX compilation failed: ${message}`;
			
			// Clear any existing PDF on error
			if (pdfDoc) {
				pdfDoc.destroy();
				pdfDoc = null;
				totalPages = 0;
				currentPage = 1;
			}
		};
		
		// Listen for compilation start (if we want to show loading state)
		const handleLatexCompiling = () => {
			isCompiling = true;
			error = null;
			compilationErrors = [];
		};
		
		window.addEventListener('latex-compiled', handleLatexCompiled as EventListener);
		window.addEventListener('latex-compile-error', handleLatexError as EventListener);
		window.addEventListener('latex-compiling', handleLatexCompiling as EventListener);
		
		// Cleanup event listeners
		return () => {
			window.removeEventListener('latex-compiled', handleLatexCompiled as EventListener);
			window.removeEventListener('latex-compile-error', handleLatexError as EventListener);
			window.removeEventListener('latex-compiling', handleLatexCompiling as EventListener);
		};
	});

	async function loadPDFJS() {
		try {
			const pdfjsLib = await import('pdfjs-dist');
			
			// Set worker path
			pdfjsLib.GlobalWorkerOptions.workerSrc = new URL(
				'pdfjs-dist/build/pdf.worker.mjs',
				import.meta.url
			).toString();

			return pdfjsLib;
		} catch (err) {
			console.error('Failed to load PDF.js:', err);
			error = 'Failed to load PDF viewer';
		}
	}

	async function loadPDF(pdfPath: string) {
		if (!pdfPath) return;

		try {
			isLoading = true;
			error = null;
			compilationErrors = [];
			
			const pdfjsLib = await loadPDFJS();
			if (!pdfjsLib) return;

			console.log('Loading PDF from:', pdfPath);
			const loadingTask = pdfjsLib.getDocument(pdfPath);
			
			// Add progress tracking
			loadingTask.onProgress = (progressData) => {
				console.log('PDF loading progress:', progressData);
			};
			
			pdfDoc = await loadingTask.promise;
			totalPages = pdfDoc.numPages;
			currentPage = 1;
			
			await renderPage();
			console.log('PDF loaded successfully:', totalPages, 'pages');
		} catch (err) {
			console.error('Error loading PDF:', err);
			
			// Provide more specific error messages
			if (err instanceof Error) {
				if (err.message.includes('404') || err.message.includes('Not Found')) {
					error = 'PDF file not found. Make sure the compilation was successful.';
				} else if (err.message.includes('network')) {
					error = 'Network error loading PDF. Please check your connection.';
				} else if (err.message.includes('InvalidPDFException')) {
					error = 'Invalid PDF file. The LaTeX compilation may have produced a corrupted file.';
				} else {
					error = `Failed to load PDF: ${err.message}`;
				}
			} else {
				error = 'Unknown error loading PDF document';
			}
		} finally {
			isLoading = false;
		}
	}

	async function renderPage() {
		if (!pdfDoc || !canvas) return;

		try {
			const page = await pdfDoc.getPage(currentPage);
			const viewport = page.getViewport({ scale, rotation });
			
			const context = canvas.getContext('2d');
			canvas.height = viewport.height;
			canvas.width = viewport.width;

			const renderContext = {
				canvasContext: context,
				viewport: viewport
			};

			await page.render(renderContext).promise;
		} catch (err) {
			console.error('Error rendering page:', err);
			error = 'Failed to render PDF page';
		}
	}

	function showPlaceholder() {
		if (!canvas) return;
		
		const ctx = canvas.getContext('2d');
		if (!ctx) return;

		canvas.width = 600;
		canvas.height = 800;
		
		// Draw placeholder
		ctx.fillStyle = '#f8f9fa';
		ctx.fillRect(0, 0, canvas.width, canvas.height);
		
		ctx.strokeStyle = '#e9ecef';
		ctx.lineWidth = 2;
		ctx.setLineDash([5, 5]);
		ctx.strokeRect(20, 20, canvas.width - 40, canvas.height - 40);
		
		ctx.fillStyle = '#6c757d';
		ctx.font = '18px system-ui';
		ctx.textAlign = 'center';
		ctx.fillText('PDF Preview', canvas.width / 2, canvas.height / 2 - 30);
		
		ctx.font = '14px system-ui';
		ctx.fillText('Open a LaTeX file and start editing.', canvas.width / 2, canvas.height / 2);
		ctx.fillText('The PDF will update automatically when you save.', canvas.width / 2, canvas.height / 2 + 20);
	}

	function nextPage() {
		if (currentPage < totalPages) {
			currentPage++;
			renderPage();
		}
	}

	function prevPage() {
		if (currentPage > 1) {
			currentPage--;
			renderPage();
		}
	}

	function zoomIn() {
		scale += 0.2;
		renderPage();
	}

	function zoomOut() {
		if (scale > 0.4) {
			scale -= 0.2;
			renderPage();
		}
	}

	function rotate() {
		rotation += 90;
		if (rotation >= 360) rotation = 0;
		renderPage();
	}

	onDestroy(() => {
		if (pdfDoc) {
			pdfDoc.destroy();
		}
	});
</script>

<div class="h-full flex flex-col {fullPanel ? 'bg-background' : 'bg-muted/20'}">
	<!-- PDF Toolbar -->
	{#if showToolbar}
	<div class="h-12 border-b bg-background flex items-center px-3 gap-2">
		<div class="flex items-center gap-1">
			<Button
				variant="ghost"
				size="sm"
				onclick={prevPage}
				disabled={!pdfDoc || currentPage <= 1}
			>
				←
			</Button>
			
			<span class="text-sm px-2">
				{#if pdfDoc}
					{currentPage} / {totalPages}
				{:else}
					- / -
				{/if}
			</span>
			
			<Button
				variant="ghost"
				size="sm"
				onclick={nextPage}
				disabled={!pdfDoc || currentPage >= totalPages}
			>
				→
			</Button>
		</div>

		<div class="h-4 w-px bg-border mx-1"></div>

		<div class="flex items-center gap-1">
			<Button variant="ghost" size="sm" onclick={zoomOut} disabled={!pdfDoc}>
				<ZoomOut size={14} />
			</Button>
			
			<span class="text-xs px-1 min-w-12 text-center">
				{Math.round(scale * 100)}%
			</span>
			
			<Button variant="ghost" size="sm" onclick={zoomIn} disabled={!pdfDoc}>
				<ZoomIn size={14} />
			</Button>
		</div>

		<div class="h-4 w-px bg-border mx-1"></div>

		<Button variant="ghost" size="sm" onclick={rotate} disabled={!pdfDoc}>
			<RotateCw size={14} />
		</Button>
	</div>
	{/if}

	<!-- PDF Viewer -->
	<div class="flex-1 overflow-auto p-0" bind:this={canvasContainer}>
		{#if isLoading}
			<div class="flex items-center justify-center h-full">
				<div class="text-muted-foreground">Loading PDF...</div>
			</div>
		{:else if isCompiling}
			<div class="flex items-center justify-center h-full">
				<div class="text-center text-muted-foreground">
					<div class="animate-spin h-8 w-8 border-2 border-blue-500 border-t-transparent rounded-full mx-auto mb-4"></div>
					<p class="text-sm">Compiling LaTeX...</p>
				</div>
			</div>
		{:else if error}
			<div class="flex items-center justify-center h-full p-4">
				<div class="text-center text-muted-foreground max-w-lg">
					<FileText size={48} class="mx-auto mb-4 opacity-50" />
					<p class="text-sm mb-4">{error}</p>
					
					{#if compilationErrors.length > 0}
						<div class="text-left bg-red-50 border border-red-200 rounded-lg p-3 mt-4">
							<h4 class="font-semibold text-red-800 mb-2">Compilation Errors:</h4>
							<ul class="text-xs text-red-700 space-y-1">
								{#each compilationErrors as err}
									<li class="font-mono">{err}</li>
								{/each}
							</ul>
						</div>
					{/if}
				</div>
			</div>
		{:else}
			<div class="flex justify-center min-h-full {fullPanel ? 'p-0' : 'p-2'}">
				<canvas 
					bind:this={canvas}
					class="max-w-full h-auto"
					style="{fullPanel ? '' : 'box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1);'}"
				></canvas>
			</div>
		{/if}
	</div>

	<!-- Status Bar -->
	{#if showToolbar}
	<div class="h-6 border-t bg-muted/50 flex items-center px-3 text-xs text-muted-foreground">
		{#if pdfDoc}
			PDF loaded • {totalPages} pages
		{:else}
			Ready for PDF preview
		{/if}
	</div>
	{/if}
</div>