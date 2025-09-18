<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { Play, Download, FileText } from '@lucide/svelte';
	import { pdfStore, latexStore, isCompiling } from '$lib/stores';
	import { billingStore } from '$lib/stores';
	import { Badge } from '$lib/components/ui/badge';
	import LaTeXErrorPanel from '$lib/components/errors/LaTeXErrorPanel.svelte';

	// Props
	interface Props {
		showToolbar?: boolean;
		fullPanel?: boolean;
	}
	
	let { showToolbar = true, fullPanel = false }: Props = $props();

	let canvases = $state<HTMLCanvasElement[]>([]);
	
	// Reactive store subscriptions
	const pdfState = $derived($pdfStore);
	const latexState = $derived($latexStore);
	const billingState = $derived($billingStore);
	let downloadUsed = $state(false);
	// Use store derivatives directly - no manual subscriptions needed
	const hasValidPdf = $derived(pdfState.currentPdf && pdfState.currentPdf.pdfDoc && !pdfState.isLoading && !pdfState.error);

	// Update canvases when PDF changes
	$effect(() => {
		if (pdfState.currentPdf && canvases.length !== pdfState.currentPdf.numPages) {
			// Initialize canvases array for all pages
			canvases = Array(pdfState.currentPdf.numPages).fill(null);
		}
	});

	// Render all pages when canvases are available
	$effect(() => {
		if (pdfState.currentPdf && canvases.length > 0 && canvases.some(c => c)) {
			renderAllPages();
		}
	});

	async function renderAllPages() {
		if (!pdfState.currentPdf?.pdfDoc) return;

		try {
				// Get device pixel ratio for high-DPI displays
				const devicePixelRatio = window.devicePixelRatio || 1;
			
			for (let pageNum = 1; pageNum <= pdfState.currentPdf.numPages; pageNum++) {
				const canvas = canvases[pageNum - 1];
				if (!canvas) continue;

				const page = await pdfState.currentPdf.pdfDoc.getPage(pageNum);
				const context = canvas.getContext('2d');
				if (!context) continue;

				// Use reasonable display scale that fits well in the viewport
				const displayScale = 1.4;
				// Render at higher internal resolution for crisp text
				const renderScale = displayScale * devicePixelRatio;
				
				// Get viewports for both display and rendering
				const displayViewport = page.getViewport({ scale: displayScale });
				const renderViewport = page.getViewport({ scale: renderScale });
				
				// Set canvas internal size (high resolution for crisp rendering)
				canvas.width = renderViewport.width;
				canvas.height = renderViewport.height;
				
				// Set CSS display size (reasonable size for viewing)
				canvas.style.width = `${displayViewport.width}px`;
				canvas.style.height = `${displayViewport.height}px`;
				
				// Enable crisp text rendering
				context.imageSmoothingEnabled = true;
				context.imageSmoothingQuality = 'high';

				await page.render({
					canvasContext: context,
					viewport: renderViewport
				}).promise;
			}
		} catch (error) {
			console.error('Failed to render PDF pages:', error);
		}
	}

	// Public method to load a PDF by path - delegates to store
	export async function loadPDFByPath(relativePath: string) {
		await pdfStore.loadPdf(relativePath);
	}

	onMount(() => {
		// PDF store handles all initialization automatically
		// through the orchestrator system
		console.log('PDFPreview: Component mounted, stores handle initialization');
		// Initialize per-session download usage flag
		downloadUsed = getDownloadUsed();
	});

	// Handler functions following STATE.md reactive patterns
	function handleCompile() {
		// Use latexStore.forceCompile() as per STATE.md
		latexStore.forceCompile();
	}

	function hasUnlimitedDownloads(): boolean {
		if (!billingState.isReady) return false;
		// Check by benefit type or description keyword
		return billingState.benefits.some((b) => b.benefit_type === 'unlimited_downloads' || b.description?.toLowerCase().includes('unlimited download'));
	}

	function getDownloadUsed(): boolean {
		if (typeof window === 'undefined') return false;
		return sessionStorage.getItem('dcmt-download-used') === '1';
	}

	async function handleDownload() {
		if (!hasValidPdf || !pdfState.currentPdf) return;

		// Enforce download limits: if user lacks unlimited_downloads and no active sub, allow only one
		if (billingState.isReady && !hasUnlimitedDownloads()) {
			// Track a single allowed download per session
			const key = 'dcmt-download-used';
			const used = sessionStorage.getItem(key);
			if (used === '1') {
				return; // Already used this session
			} else {
				sessionStorage.setItem(key, '1');
				// Reflect immediately in UI
				downloadUsed = true;
			}
		}
		
		try {
			// Create download link for PDF
			const link = document.createElement('a');
			link.href = pdfState.currentPdf.url;
			link.download = pdfState.currentPdf.path.split('/').pop() || 'document.pdf';
			document.body.appendChild(link);
			link.click();
			document.body.removeChild(link);
		} catch (error) {
			console.error('Failed to download PDF:', error);
		}
	}

	onDestroy(() => {
		// Store cleanup is handled by the orchestrator
		console.log('PDFPreview: Component destroyed');
	});
</script>

<div class="h-full flex flex-col {fullPanel ? 'bg-background' : 'bg-muted/20'}">
	<!-- PDF Toolbar -->
	{#if showToolbar}
	<div class="h-8 border-b bg-background flex items-center px-3 gap-2">
		<Button
			variant="default"
			size="sm"
			onclick={handleCompile}
			disabled={$isCompiling}
			class="h-6 gap-1.5 px-2"
		>
			<Play size={12} />
			{$isCompiling ? 'Compiling...' : 'Compile'}
		</Button>

		<Button
			variant="outline"
			size="sm"
			onclick={handleDownload}
			disabled={!hasValidPdf || (billingState.isReady && !hasUnlimitedDownloads() && downloadUsed)}
			class="h-6 gap-1.5 px-2"
		>
			<Download size={12} />
			Download
		</Button>

		{#if billingState.isReady && !hasUnlimitedDownloads()}
			<div class="ml-auto flex items-center gap-2 text-xs text-muted-foreground">
				<Badge variant="secondary">Downloads</Badge>
				{#if downloadUsed}
					<span>None remaining</span>
					<a href="https://polar.sh/" target="_blank" class="underline underline-offset-2 hover:text-foreground">Upgrade to get more</a>
				{:else}
					<span>1 remaining</span>
				{/if}
			</div>
		{/if}
	</div>
	{/if}

	<!-- PDF Viewer -->
	<div class="flex-1 overflow-auto p-4 relative">
		{#if hasValidPdf && pdfState.currentPdf}
			<!-- Render all PDF pages in scrollable container -->
			<div class="flex flex-col items-center gap-4">
				{#each Array(pdfState.currentPdf.numPages) as _, pageIndex}
					<canvas 
						bind:this={canvases[pageIndex]}
						class="max-w-full h-auto border border-border rounded-lg shadow-sm"
						style="box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1);"
					></canvas>
				{/each}
			</div>
		{/if}

		<!-- Overlay states when PDF is not ready -->
		{#if pdfState.isLoading}
			<div class="absolute inset-0 flex items-center justify-center bg-background">
				<div class="text-center text-muted-foreground">
					<div class="animate-spin h-8 w-8 border-2 border-blue-500 border-t-transparent rounded-full mx-auto mb-4"></div>
					<p class="text-sm">Loading PDF... {pdfState.loadingProgress}%</p>
				</div>
			</div>
		{:else if $isCompiling}
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

	<!-- LaTeX Error Panel - Always at bottom when errors exist -->
	<LaTeXErrorPanel />
</div>