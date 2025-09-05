<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { ZoomIn, ZoomOut, RotateCw, Download, FileText } from '@lucide/svelte';

	let canvasContainer: HTMLDivElement;
	let canvas: HTMLCanvasElement;
	let pdfDoc: any = null;
	let currentPage = $state(1);
	let totalPages = $state(0);
	let scale = $state(1.5);
	let rotation = $state(0);
	let isLoading = $state(false);
	let error = $state<string | null>(null);

	const mockPdfPath = '/sample.pdf'; // This would be generated from LaTeX compilation

	onMount(async () => {
		await loadPDFJS();
		// Load a sample PDF or show placeholder
		showPlaceholder();
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
			
			const pdfjsLib = await loadPDFJS();
			if (!pdfjsLib) return;

			const loadingTask = pdfjsLib.getDocument(pdfPath);
			pdfDoc = await loadingTask.promise;
			totalPages = pdfDoc.numPages;
			currentPage = 1;
			
			await renderPage();
		} catch (err) {
			console.error('Error loading PDF:', err);
			error = 'Failed to load PDF document';
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
		ctx.fillText('PDF Preview', canvas.width / 2, canvas.height / 2 - 20);
		
		ctx.font = '14px system-ui';
		ctx.fillText('Compile your LaTeX document to see the preview', canvas.width / 2, canvas.height / 2 + 10);
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

<div class="h-full flex flex-col bg-muted/20">
	<!-- PDF Toolbar -->
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

	<!-- PDF Viewer -->
	<div class="flex-1 overflow-auto p-4" bind:this={canvasContainer}>
		{#if isLoading}
			<div class="flex items-center justify-center h-full">
				<div class="text-muted-foreground">Loading PDF...</div>
			</div>
		{:else if error}
			<div class="flex items-center justify-center h-full">
				<div class="text-center text-muted-foreground">
					<FileText size={48} class="mx-auto mb-4 opacity-50" />
					<p class="text-sm">{error}</p>
				</div>
			</div>
		{:else}
			<div class="flex justify-center">
				<canvas 
					bind:this={canvas}
					class="shadow-lg border border-border/50"
					style="max-width: 100%; height: auto;"
				></canvas>
			</div>
		{/if}
	</div>

	<!-- Status Bar -->
	<div class="h-6 border-t bg-muted/50 flex items-center px-3 text-xs text-muted-foreground">
		{#if pdfDoc}
			PDF loaded • {totalPages} pages
		{:else}
			Ready for PDF preview
		{/if}
	</div>
</div>