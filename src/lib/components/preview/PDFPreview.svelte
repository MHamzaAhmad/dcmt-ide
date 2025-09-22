<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { Tooltip as TooltipRoot, TooltipContent, TooltipTrigger, TooltipProvider } from '$lib/components/ui/tooltip';
	import { pdfStore, latexStore, isCompiling } from '$lib/stores';
	import { LaTeXProvider } from '$lib/api/types';
	import { billingStore } from '$lib/stores';
	// centralized upgrade flow via billingStore
	import { Badge } from '$lib/components/ui/badge';
	import LaTeXErrorPanel from '$lib/components/errors/LaTeXErrorPanel.svelte';
	import { FileText } from '@lucide/svelte';

	// Props
	interface Props {
		showToolbar?: boolean;
		fullPanel?: boolean;
	}
	
	let { showToolbar = true, fullPanel = false }: Props = $props();

	let canvases = $state<HTMLCanvasElement[]>([]);
	let containerEl = $state<HTMLDivElement | null>(null);
	let resizeObserver: ResizeObserver | null = null;
	let renderQueued = $state(false);
	let onWindowResize: (() => void) | null = null;
	let resizeTimer: number | null = null;
	// Track per-page render tasks to cancel in-flight renders before re-rendering
	let renderTasks = $state<Array<{ cancel: () => void; promise: Promise<unknown> } | null>>([]);
	let isRendering = $state(false);
	let rerenderPending = $state(false);
	let lastContainerWidth = $state(0);
	let pageRendered = $state<boolean[]>([]);
	let lastDocKey = $state<string | null>(null);

	function getEffectiveContainerWidth(): number {
		const el = containerEl;
		if (!el) return 0;
		// Use border-box width to avoid clientWidth changes due to scrollbars
		const rectWidth = el.getBoundingClientRect().width;
		const cs = getComputedStyle(el);
		const padL = parseFloat(cs.paddingLeft || '0');
		const padR = parseFloat(cs.paddingRight || '0');
		const borderL = parseFloat(cs.borderLeftWidth || '0');
		const borderR = parseFloat(cs.borderRightWidth || '0');
		// Convert to content width: border-box - borders - padding
		const contentWidth = Math.max(0, rectWidth - borderL - borderR - padL - padR);
		return contentWidth;
	}
	
	// Reactive store subscriptions
	const pdfState = $derived($pdfStore);
	const latexState = $derived($latexStore);
	const billingState = $derived($billingStore);
	let downloadUsed = $state(false);
	let upgrading = $state(false);
	// Use store derivatives directly - no manual subscriptions needed
	const hasValidPdf = $derived(pdfState.currentPdf && pdfState.currentPdf.pdfDoc && !pdfState.isLoading && !pdfState.error);

	onMount(() => {
		downloadUsed = getDownloadUsed();

		// Observe container size to re-render pages responsively
		if (typeof ResizeObserver !== 'undefined') {
			resizeObserver = new ResizeObserver((entries) => {
				const entry = entries[0];
				if (!entry) return;
				const width = getEffectiveContainerWidth();
				if (Math.abs(width - lastContainerWidth) >= 1) {
					queueResizeRender(width);
				}
			});
			if (containerEl) resizeObserver.observe(containerEl);
		}

		// Also re-render on window resize or zoom (DPR changes)
		onWindowResize = () => {
			const width = getEffectiveContainerWidth();
			if (Math.abs(width - lastContainerWidth) >= 1) {
				queueResizeRender(width);
			}
		};
		window.addEventListener('resize', onWindowResize);
	});

	function handleCompile() { latexStore.forceCompile(); }

	// Provider selection (default Auto)
	const providerOptions: { label: string; value: LaTeXProvider }[] = [
		{ label: 'Auto', value: LaTeXProvider.Auto },
		{ label: 'pdfLaTeX', value: LaTeXProvider.Pdflatex },
		{ label: 'XeLaTeX', value: LaTeXProvider.Xelatex },
		{ label: 'LuaLaTeX', value: LaTeXProvider.Lualatex }
	];
	function onProviderChange(ev: Event) {
		const v = (ev.target as HTMLSelectElement).value as LaTeXProvider;
		latexStore.setProvider(v);
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
			// Cancel any in-flight tasks from previous doc/pages
			for (const t of renderTasks) { try { t?.cancel(); } catch {} }
			renderTasks = [];
			// Initialize canvases array for all pages
			canvases = Array(pdfState.currentPdf.numPages).fill(null);
			// Initialize/resize render tasks array accordingly
			renderTasks = Array(pdfState.currentPdf.numPages).fill(null);
			pageRendered = Array(pdfState.currentPdf.numPages).fill(false);
			lastDocKey = pdfState.currentPdf.path || String(Date.now());
		}
	});

	// Ensure observer attaches when container becomes available later
	$effect(() => {
		if (resizeObserver && containerEl) {
			try { resizeObserver.observe(containerEl); } catch {}
		}
	});

	// Render all pages when canvases are available
	$effect(() => {
		if (pdfState.currentPdf && canvases.length > 0 && canvases.some(c => c)) {
			// Set baseline container width and trigger initial render once
			if (lastContainerWidth === 0) lastContainerWidth = getEffectiveContainerWidth();
			scheduleRender();
		}
	});

	function scheduleRender() {
		if (isRendering) {
			rerenderPending = true;
			return;
		}
		if (renderQueued) return;
		renderQueued = true;
		requestAnimationFrame(async () => {
			renderQueued = false;
			await renderAllPages();
			if (rerenderPending) {
				rerenderPending = false;
				scheduleRender();
			}
		});
	}

	async function renderAllPages() {
		if (!pdfState.currentPdf?.pdfDoc) return;

		try {
				isRendering = true;
				// Get device pixel ratio for high-DPI displays
				const devicePixelRatio = window.devicePixelRatio || 1;
			const containerWidth = getEffectiveContainerWidth();
			if (!containerWidth || containerWidth <= 0) return;
			
			for (let pageNum = 1; pageNum <= pdfState.currentPdf.numPages; pageNum++) {
				const canvas = canvases[pageNum - 1];
				if (!canvas) continue;

				const page = await pdfState.currentPdf.pdfDoc.getPage(pageNum);
				const context = canvas.getContext('2d');
				if (!context) continue;

				// If a render is in progress for this page, cancel and await its completion
				const existingTask = renderTasks[pageNum - 1];
				if (existingTask) {
					try { existingTask.cancel(); } catch {}
					try { await existingTask.promise; } catch {}
					renderTasks[pageNum - 1] = null;
				}

				// Compute a display scale that fits the current container width
				const unscaledViewport = page.getViewport({ scale: 1 });
				const displayScale = containerWidth / unscaledViewport.width;
				// Render at higher internal resolution for crisp text
				const renderScale = displayScale * devicePixelRatio;
				
				// Get viewports for both display and rendering
				const displayViewport = page.getViewport({ scale: displayScale });
				const renderViewport = page.getViewport({ scale: renderScale });
				
				// Prepare offscreen canvas to render into, then blit to onscreen to avoid visible clears
				const targetW = Math.floor(renderViewport.width);
				const targetH = Math.floor(renderViewport.height);
				const sizeChanged = canvas.width !== targetW || canvas.height !== targetH;
				let offscreen: HTMLCanvasElement | OffscreenCanvas;
				if ('OffscreenCanvas' in window) {
					// @ts-ignore - TS may not know OffscreenCanvas on Window
					offscreen = new OffscreenCanvas(targetW, targetH);
				} else {
					const tmp = document.createElement('canvas');
					tmp.width = targetW;
					tmp.height = targetH;
					offscreen = tmp;
				}
				
				// Do not set CSS width/height per render; use responsive CSS to avoid resize loops
				
				// Enable crisp text rendering for final blit
				context.imageSmoothingEnabled = true;
				context.imageSmoothingQuality = 'high';

				// If nothing changed and page already rendered for this doc, skip
				if (!sizeChanged && pageRendered[pageNum - 1] && lastDocKey === (pdfState.currentPdf.path || lastDocKey)) {
					continue;
				}

				const offCtx = (offscreen as any).getContext('2d');
				if (!offCtx) continue;
				(offCtx as CanvasRenderingContext2D).imageSmoothingEnabled = true;
				(offCtx as CanvasRenderingContext2D).imageSmoothingQuality = 'high';
				const task = page.render({
					canvasContext: offCtx,
					viewport: renderViewport
				});
				renderTasks[pageNum - 1] = task as unknown as { cancel: () => void; promise: Promise<unknown> };
				await task.promise;
				// Update onscreen canvas size only once we have a rendered frame, then blit
				if (sizeChanged) {
					canvas.width = targetW;
					canvas.height = targetH;
				}
				if ('transferToImageBitmap' in offscreen) {
					// @ts-ignore
					const bitmap = (offscreen as OffscreenCanvas).transferToImageBitmap();
					context.clearRect(0, 0, canvas.width, canvas.height);
					// Draw bitmap at 1:1
					context.drawImage(bitmap as any, 0, 0);
				} else {
					context.clearRect(0, 0, canvas.width, canvas.height);
					context.drawImage(offscreen as HTMLCanvasElement, 0, 0);
				}
				pageRendered[pageNum - 1] = true;
			}
		} catch (error) {
			console.error('Failed to render PDF pages:', error);
		} finally {
			isRendering = false;
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

	onDestroy(() => {
		// Store cleanup is handled by the orchestrator
		console.log('PDFPreview: Component destroyed');
		if (resizeObserver && containerEl) {
			try { resizeObserver.unobserve(containerEl); } catch {}
		}
		resizeObserver = null;
		if (onWindowResize) {
			window.removeEventListener('resize', onWindowResize);
			onWindowResize = null;
		}
		if (resizeTimer) {
			clearTimeout(resizeTimer);
			resizeTimer = null;
		}
		// Cancel any in-flight render tasks
		for (const t of renderTasks) {
			try { t?.cancel(); } catch {}
		}
	});

	function queueResizeRender(width: number) {
		lastContainerWidth = width;
		if (resizeTimer) {
			clearTimeout(resizeTimer);
		}
		resizeTimer = window.setTimeout(() => {
			resizeTimer = null;
			scheduleRender();
		}, 120);
	}
</script>

<div class="h-full flex flex-col {fullPanel ? 'bg-background' : 'bg-muted/20'}">
	<!-- Single Header Bar -->
	<div class="h-8 border-b bg-background flex items-center px-3 gap-3">
		<!-- Left: engine + actions -->
		<div class="flex items-center gap-2">
			<!-- Engine dropdown with tooltip (before Compile) -->
			<TooltipProvider>
				<TooltipRoot>
					<TooltipTrigger>
						<select
							id="latex-provider-select"
							class="text-xs border rounded px-2 py-1 bg-background w-28 truncate"
							onchange={onProviderChange}
							bind:value={latexState.selectedProvider}
							title="Select LaTeX engine"
						>
							{#each providerOptions as opt}
								<option value={opt.value}>{opt.label}</option>
							{/each}
						</select>
					</TooltipTrigger>
					<TooltipContent>Engine</TooltipContent>
				</TooltipRoot>
			</TooltipProvider>
			<Button variant="default" size="sm" onclick={handleCompile} disabled={$isCompiling} class="h-6 gap-1.5 px-2">
				{$isCompiling ? 'Compiling…' : 'Compile'}
			</Button>
			<Button variant="outline" size="sm" onclick={handleDownload} disabled={!hasValidPdf || (billingState.isReady && !hasUnlimitedDownloads() && downloadUsed)} class="h-6 gap-1.5 px-2">
				Download
			</Button>
		</div>
		<!-- Spacer -->
		<div class="flex-1"></div>
			<!-- Right: provider, checkpoints dropdown and downloads badge -->
		<div class="flex items-center gap-2">
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
	<div class="flex-1 overflow-auto p-4 relative" bind:this={containerEl}>
		{#if hasValidPdf && pdfState.currentPdf}
			<!-- Render all PDF pages in scrollable container -->
			<div class="flex flex-col items-center gap-4">
				{#each Array(pdfState.currentPdf.numPages) as _, pageIndex}
					<canvas bind:this={canvases[pageIndex]} class="shadow rounded bg-white w-full h-auto block"></canvas>
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