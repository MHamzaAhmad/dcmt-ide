<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { projectStore } from '$lib/stores';
	import { FolderOpen, Settings, Code2 } from '@lucide/svelte';

	// Get reactive project state
	const projectState = $derived($projectStore);

	// Handle project selection
	async function handleSelectProject() {
		console.log('handleSelectProject called');
		try {
			console.log('Calling projectStore.selectProject()');
			const result = await projectStore.selectProject();
			console.log('Project selection result:', result);
		} catch (error) {
			console.error('Failed to select project:', error);
		}
	}

	// Show loading state
	let isSelecting = $derived(projectState.isLoading);
	let hasError = $derived(!!projectState.error);
	let errorMessage = $derived(projectState.error);
</script>

<div class="min-h-screen flex items-center justify-center bg-gradient-to-br from-background to-muted/50 p-4">
	<div class="w-full max-w-2xl space-y-8">
		<!-- Header -->
		<div class="text-center space-y-4">
			<div class="flex justify-center">
				<div class="h-16 w-16 rounded-2xl bg-primary/10 flex items-center justify-center">
					<Code2 class="h-8 w-8 text-primary" />
				</div>
			</div>
			<h1 class="text-3xl font-bold tracking-tight">Welcome to Researgent Editor</h1>
			<p class="text-xl text-muted-foreground max-w-md mx-auto">
				A powerful editor with integrated AI for your PDF creation workflow.
			</p>
		</div>

		<!-- Main Card -->
		<Card class="border-2 border-dashed border-border/50 hover:border-border transition-colors">
			<CardHeader class="text-center pb-4">
				<div class="flex justify-center mb-4">
					<div class="h-12 w-12 rounded-xl bg-primary/10 flex items-center justify-center">
						<FolderOpen class="h-6 w-6 text-primary" />
					</div>
				</div>
				<CardTitle class="text-xl">Select Your Project</CardTitle>
				<CardDescription class="text-base">
					Choose a folder to start working with your project files.
				</CardDescription>
			</CardHeader>
			<CardContent class="space-y-6">
				<!-- Project Selection Button -->
				<div class="flex flex-col items-center space-y-4">
					<Button 
						size="lg" 
						onclick={handleSelectProject}
						disabled={isSelecting}
						class="w-full max-w-xs h-12"
					>
						{#if isSelecting}
							<div class="h-4 w-4 animate-spin rounded-full border-2 border-background border-t-transparent mr-2"></div>
							Selecting...
						{:else}
							<FolderOpen class="h-5 w-5 mr-2" />
							Select Project Folder
						{/if}
					</Button>

					{#if hasError}
						<div class="text-sm text-destructive text-center">
							{errorMessage || 'Failed to select project folder'}
						</div>
					{/if}
				</div>

				<!-- Features Preview -->
				<div class="border-t pt-6">
					<h3 class="text-sm font-medium text-center mb-4 text-muted-foreground">
						What you'll get:
					</h3>
					<div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
						<div class="text-center space-y-2">
							<div class="h-8 w-8 rounded-lg bg-blue-500/10 flex items-center justify-center mx-auto">
								<Code2 class="h-4 w-4 text-blue-500" />
							</div>
							<div class="font-medium">Code Editor</div>
							<div class="text-muted-foreground">Syntax highlighting and IntelliSense</div>
						</div>
						<div class="text-center space-y-2">
							<div class="h-8 w-8 rounded-lg bg-green-500/10 flex items-center justify-center mx-auto">
								<FolderOpen class="h-4 w-4 text-green-500" />
							</div>
							<div class="font-medium">File Explorer</div>
							<div class="text-muted-foreground">Navigate and manage your files</div>
						</div>
						<div class="text-center space-y-2">
							<div class="h-8 w-8 rounded-lg bg-purple-500/10 flex items-center justify-center mx-auto">
								<Settings class="h-4 w-4 text-purple-500" />
							</div>
							<div class="font-medium">AI Chat</div>
							<div class="text-muted-foreground">Chat with AI to delegate the work to it</div>
						</div>
					</div>
				</div>
			</CardContent>
		</Card>

		<!-- Footer -->
		<div class="text-center text-sm text-muted-foreground">
			<p>Your files remain on your device. Researgent Editor provides a secure, local environment with AI.</p>
		</div>
	</div>
</div>