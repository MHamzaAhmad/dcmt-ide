<!--
  StoreInitializer - Initializes the reactive store system
  This component should be included early in the app lifecycle
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';
  import { orchestrator } from '$lib/stores';
  import { useQueryClient } from '@tanstack/svelte-query';

  interface Props {
    rootPath?: string;
    skipAgent?: boolean;
    showProgress?: boolean;
  }

  let { rootPath = '', skipAgent = false, showProgress = false }: Props = $props();

  // Get orchestrator state
  const orchestratorState = $derived($orchestrator);
  
  // Initialization status
  let initError = $state<string | null>(null);
  let isInitialized = $state(false);

  onMount(async () => {
    if (!browser) return;

    console.log('StoreInitializer: Starting system initialization...');

    try {
      // Get query client from context
      let queryClient;
      try {
        queryClient = useQueryClient();
      } catch (e) {
        console.debug('StoreInitializer: No query client available in context');
      }

      // Initialize the system
      await orchestrator.initialize({
        rootPath,
        queryClient,
        skipAgentInit: skipAgent,
        retryOnFailure: true
      });

      isInitialized = true;
      console.log('StoreInitializer: System initialization completed successfully');

    } catch (error) {
      initError = error instanceof Error ? error.message : 'System initialization failed';
      console.error('StoreInitializer: System initialization failed:', error);
    }
  });

  // Listen for global initialization events
  function handleInitReady(event: Event) {
    const customEvent = event as CustomEvent;
    console.log('StoreInitializer: System ready event received:', customEvent.detail);
    isInitialized = true;
  }

  function handleInitError(event: Event) {
    const customEvent = event as CustomEvent;
    initError = customEvent.detail.error;
    console.error('StoreInitializer: System error event received:', customEvent.detail);
  }

  if (browser) {
    window.addEventListener('dcmt:ready', handleInitReady);
    window.addEventListener('dcmt:init-error', handleInitError);
  }
</script>

{#if showProgress && orchestratorState.isInitializing}
  <div class="fixed inset-0 bg-background/80 backdrop-blur-sm z-50 flex items-center justify-center">
    <div class="bg-card border rounded-lg p-6 w-full max-w-md">
      <h2 class="text-lg font-semibold mb-4">Initializing DCMT Editor...</h2>
      
      <div class="space-y-3 mb-4">
        {#each orchestratorState.steps as step, index}
          <div class="flex items-center gap-3">
            {#if step.status === 'completed'}
              <div class="w-4 h-4 bg-green-500 rounded-full flex items-center justify-center">
                <svg class="w-2 h-2 text-white" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
                </svg>
              </div>
            {:else if step.status === 'running'}
              <div class="w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
            {:else if step.status === 'failed'}
              <div class="w-4 h-4 bg-red-500 rounded-full flex items-center justify-center">
                <svg class="w-2 h-2 text-white" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
                </svg>
              </div>
            {:else}
              <div class="w-4 h-4 border-2 border-muted rounded-full"></div>
            {/if}
            
            <div class="flex-1">
              <div class="text-sm font-medium">{step.description}</div>
              {#if step.error}
                <div class="text-xs text-red-600">{step.error}</div>
              {:else if step.duration}
                <div class="text-xs text-muted-foreground">{step.duration}ms</div>
              {/if}
            </div>
          </div>
        {/each}
      </div>
      
      <div class="text-xs text-muted-foreground text-center">
        Progress: {orchestrator.getInitializationProgress()}%
      </div>
    </div>
  </div>
{/if}

{#if initError}
  <div class="fixed inset-0 bg-background/80 backdrop-blur-sm z-50 flex items-center justify-center">
    <div class="bg-card border border-red-200 rounded-lg p-6 w-full max-w-md">
      <h2 class="text-lg font-semibold text-red-800 mb-4">Initialization Failed</h2>
      <p class="text-sm text-muted-foreground mb-4">{initError}</p>
      <button 
        class="w-full px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 transition-colors"
        onclick={() => window.location.reload()}
      >
        Reload Page
      </button>
    </div>
  </div>
{/if}