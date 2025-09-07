<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { theme } from '$lib/stores/theme.js';
	import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';
	import StoreInitializer from '$lib/components/system/StoreInitializer.svelte';
	
	let { children } = $props();
	
	const queryClient = new QueryClient({
		defaultOptions: {
			queries: {
				staleTime: 1000 * 60 * 5, // 5 minutes
				refetchOnWindowFocus: false,
			},
		},
	});

	onMount(() => {
		theme.init();
	});
</script>

<QueryClientProvider client={queryClient}>
	<StoreInitializer rootPath="" showProgress={true} />
	{@render children()}
</QueryClientProvider>
