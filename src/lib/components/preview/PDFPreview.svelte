<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { ZoomIn, ZoomOut, RotateCw, Download, FileText } from '@lucide/svelte';
	import { pdfStore, latexStore } from '$lib/stores';

	// Props
	interface Props {
		showToolbar?: boolean;
		fullPanel?: boolean;
	}
	
	let { showToolbar = true, fullPanel = false }: Props = $props();

	let canvasContainer: HTMLDivElement;
	let canvas = $state<HTMLCanvasElement>();
	
	// Reactive store subscriptions
	const pdfState = $derived($pdfStore);
	const latexState = $derived($latexStore);
	// Use store derivatives directly - no manual subscriptions needed
	const hasValidPdf = $derived(pdfState.currentPdf && pdfState.currentPdf.pdfDoc && !pdfState.isLoading && !pdfState.error);
	const canRender = $derived(pdfState.currentPdf?.pdfDoc && pdfState.canvas && pdfState.context && !pdfState.isRendering);
	const currentPageInfo = $derived({
		current: pdfState.viewer.currentPage,
		total: pdfState.currentPdf?.numPages || 0,
		text: pdfState.currentPdf ? `${pdfState.viewer.currentPage} / ${pdfState.currentPdf.numPages}` : '- / -'
	});

	// Canvas reactive effect - connect canvas to PDF store
	$effect(() => {
		if (canvas) {
			console.log('PDFPreview: Setting canvas in PDF store');
			pdfStore.setCanvas(canvas);
		} else {
			console.log('PDFPreview: Canvas not available yet');
		}
	});

	// The PDF store handles all rendering automatically when canvas is set
	// and when PDF loads. No need for additional auto-render triggers here.

	// Public method to load a PDF by path - delegates to store
	export async function loadPDFByPath(relativePath: string) {
		await pdfStore.loadPdf(relativePath);
	}

	onMount(() => {
		// PDF store handles all initialization automatically
		// through the orchestrator system
		console.log('PDFPreview: Component mounted, stores handle initialization');
	});

	// Viewer control functions - delegate to store
	function nextPage() {
		pdfStore.nextPage();
	}

	function prevPage() {
		pdfStore.prevPage();
	}

	function zoomIn() {
		pdfStore.zoomIn();
	}

	function zoomOut() {
		pdfStore.zoomOut();
	}

	function rotate() {
		pdfStore.rotate();
	}

	onDestroy(() => {
		// Store cleanup is handled by the orchestrator
		console.log('PDFPreview: Component destroyed');
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
				disabled={!hasValidPdf || currentPageInfo.current <= 1}
			>
				←
			</Button>
			
			<span class="text-sm px-2">
				{currentPageInfo.text}
			</span>
			
			<Button
				variant="ghost"
				size="sm"
				onclick={nextPage}
				disabled={!hasValidPdf || currentPageInfo.current >= currentPageInfo.total}
			>
				→
			</Button>
		</div>

		<div class="h-4 w-px bg-border mx-1"></div>

		<div class="flex items-center gap-1">
			<Button variant="ghost" size="sm" onclick={zoomOut} disabled={!hasValidPdf}>
				<ZoomOut size={14} />
			</Button>
			
			<span class="text-xs px-1 min-w-12 text-center">
				{Math.round(pdfState.viewer.scale * 100)}%
			</span>
			
			<Button variant="ghost" size="sm" onclick={zoomIn} disabled={!hasValidPdf}>
				<ZoomIn size={14} />
			</Button>
		</div>

		<div class="h-4 w-px bg-border mx-1"></div>

		<Button variant="ghost" size="sm" onclick={rotate} disabled={!hasValidPdf}>
			<RotateCw size={14} />
		</Button>
	</div>
	{/if}

	<!-- PDF Viewer -->
	<div class="flex-1 overflow-auto p-0 relative" bind:this={canvasContainer}>
		<!-- Always render canvas so it's available for PDF rendering -->
		<canvas 
			bind:this={canvas}
			class="max-w-full h-auto {hasValidPdf ? '' : 'invisible'}"
			class:absolute={!hasValidPdf}
			class:inset-0={!hasValidPdf}
			style="{fullPanel ? '' : 'box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1);'}"
		></canvas>

		<!-- Overlay states when PDF is not ready -->
		{#if pdfState.isLoading}
			<div class="absolute inset-0 flex items-center justify-center bg-background">
				<div class="text-center text-muted-foreground">
					<div class="animate-spin h-8 w-8 border-2 border-blue-500 border-t-transparent rounded-full mx-auto mb-4"></div>
					<p class="text-sm">Loading PDF... {pdfState.loadingProgress}%</p>
				</div>
			</div>
		{:else if latexState.isCompiling}
			<div class="absolute inset-0 flex items-center justify-center bg-background">
				<div class="text-center text-muted-foreground">
					<div class="animate-spin h-8 w-8 border-2 border-blue-500 border-t-transparent rounded-full mx-auto mb-4"></div>
					<p class="text-sm">Compiling LaTeX...</p>
				</div>
			</div>
		{:else if pdfState.error}
			<div class="absolute inset-0 flex items-center justify-center bg-background p-4">
				<div class="text-center text-muted-foreground max-w-lg">
					<FileText size={48} class="mx-auto mb-4 opacity-50" />
					<p class="text-sm mb-4">{pdfState.error}</p>
					
					{#if (latexState.lastCompilation?.errors?.length || 0) > 0}
						<div class="text-left bg-red-50 border border-red-200 rounded-lg p-3 mt-4">
							<h4 class="font-semibold text-red-800 mb-2">Compilation Errors:</h4>
							<ul class="text-xs text-red-700 space-y-1">
								{#each (latexState.lastCompilation?.errors || []) as err}
									<li class="font-mono">{err}</li>
								{/each}
							</ul>
						</div>
					{/if}
				</div>
			</div>
		{:else if !hasValidPdf}
			<div class="absolute inset-0 flex items-center justify-center bg-background p-4">
				<div class="text-center text-muted-foreground max-w-lg">
					<FileText size={48} class="mx-auto mb-4 opacity-50" />
					<p class="text-sm mb-2">PDF Preview</p>
					<p class="text-xs">Open a LaTeX file and start editing.</p>
					<p class="text-xs">The PDF will update automatically when you save.</p>
				</div>
			</div>
		{/if}
	</div>
</div>