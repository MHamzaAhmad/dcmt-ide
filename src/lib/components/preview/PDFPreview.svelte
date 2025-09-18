<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { Tooltip as TooltipRoot, TooltipContent, TooltipTrigger, TooltipProvider } from '$lib/components/ui/tooltip';
	import { pdfStore, latexStore, isCompiling, checkpointStore } from '$lib/stores';
	import { billingStore } from '$lib/stores';
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
	const cpState = $derived($checkpointStore);
	let downloadUsed = $state(false);
	// Use store derivatives directly - no manual subscriptions needed
	const hasValidPdf = $derived(pdfState.currentPdf && pdfState.currentPdf.pdfDoc && !pdfState.isLoading && !pdfState.error);

	const cpSelectId = 'checkpoint-select';

	onMount(() => {
		checkpointStore.list(cpState.activeNamespace).catch(console.error);
		downloadUsed = getDownloadUsed();
	});
	function onCheckpointChange(ev: Event) {
		const id = (ev.target as HTMLSelectElement).value;
		checkpointStore.select(id || null);
	}
	function handleCompile() { latexStore.forceCompile(); }
	function hasUnlimitedDownloads(): boolean {
		if (!billingState.isReady) return false;
		// Check by benefit type or description keyword
		return billingState.benefits.some((b) => b.benefit_type === 'unlimited_downloads' || b.description?.toLowerCase().includes('unlimited download'));
	}
	function getDownloadUsed(): boolean {
		if (typeof window === 'undefined') return false;
		return sessionStorage.getItem('dcmt-download-used') === '1';
	}
	function remainingDownloads(): number | '∞' {
		if (!billingState.isReady) return 0;
		if (hasUnlimitedDownloads()) return '∞';
		return downloadUsed ? 0 : 1;
	}
	async function handleDownload() {
		if (!hasValidPdf || !pdfState.currentPdf) return;
		if (billingState.isReady && !hasUnlimitedDownloads()) {
			const key = 'dcmt-download-used';
			const used = sessionStorage.getItem(key);
			if (used === '1') { return; } else { sessionStorage.setItem(key, '1'); downloadUsed = true; }
		}
		try {
			const link = document.createElement('a');
			link.href = pdfState.currentPdf.url;
			link.download = pdfState.currentPdf.path.split('/').pop() || 'document.pdf';
			document.body.appendChild(link); link.click(); document.body.removeChild(link);
		} catch (error) { console.error('Failed to download PDF:', error); }
	}

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

	onDestroy(() => {
		// Store cleanup is handled by the orchestrator
		console.log('PDFPreview: Component destroyed');
	});
</script>

<div class="h-full flex flex-col {fullPanel ? 'bg-background' : 'bg-muted/20'}">
	<!-- Single Header Bar -->
	<div class="h-8 border-b bg-background flex items-center px-3 gap-3">
		<!-- Left: actions -->
		<div class="flex items-center gap-2">
			<Button variant="default" size="sm" onclick={handleCompile} disabled={$isCompiling} class="h-6 gap-1.5 px-2">
				{$isCompiling ? 'Compiling…' : 'Compile'}
			</Button>
			<Button variant="outline" size="sm" onclick={handleDownload} disabled={!hasValidPdf || (billingState.isReady && !hasUnlimitedDownloads() && downloadUsed)} class="h-6 gap-1.5 px-2">
				Download
			</Button>
		</div>
		<!-- Spacer -->
		<div class="flex-1"></div>
		<!-- Right: checkpoints dropdown and downloads badge -->
		<div class="flex items-center gap-2">
			{#if cpState.isLoading}
				<span class="text-xs text-muted-foreground">Loading…</span>
			{:else if cpState.list.length === 0}
				<span class="text-xs text-muted-foreground">No checkpoints</span>
			{:else}
				<label class="text-xs text-muted-foreground" for={cpSelectId}>Checkpoint</label>
				<select id={cpSelectId} class="text-xs border rounded px-2 py-1 bg-background" onchange={onCheckpointChange} value={cpState.selectedId ?? ''} title="Select checkpoint">
					<option value="">Select checkpoint…</option>
					{#each cpState.list as cp}
						<option value={cp.id}>{cp.title}</option>
					{/each}
				</select>
			{/if}
			<TooltipProvider>
				<TooltipRoot>
					<TooltipTrigger>
						<div class="w-6 h-6 rounded-full border flex items-center justify-center text-xs select-none">
							{remainingDownloads()}
						</div>
					</TooltipTrigger>
					<TooltipContent>
						{hasUnlimitedDownloads() ? 'Unlimited downloads' : (downloadUsed ? 'No downloads remaining' : '1 download remaining')}
					</TooltipContent>
				</TooltipRoot>
			</TooltipProvider>
		</div>
	</div>

	<!-- PDF Viewer -->
	<div class="flex-1 overflow-auto p-4 relative">
		{#if hasValidPdf && pdfState.currentPdf}
			<div class="flex flex-col items-center gap-4">
				{#each Array(pdfState.currentPdf.numPages) as _, pageIndex}
					<canvas bind:this={canvases[pageIndex]} class="shadow rounded bg-white"></canvas>
				{/each}
			</div>
		{:else}
			<div class="h-full flex items-center justify-center text-muted-foreground">No PDF loaded</div>
		{/if}
	</div>

	<!-- Error panel -->
	<LaTeXErrorPanel />
</div>