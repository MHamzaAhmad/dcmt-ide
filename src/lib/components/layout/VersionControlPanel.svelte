<script lang="ts">
	import { GitBranch, Clock, AlertCircle } from '@lucide/svelte';
</script>

<div class="h-full flex flex-col">
	<div class="px-3 py-1 border-b">
		<h3 class="text-sm font-medium">Version Control</h3>
	</div>
	
	<div class="flex-1 flex items-center justify-center p-6">
		<div class="text-center space-y-4 max-w-xs">
			<div class="flex items-center justify-center">
				<div class="relative">
					<GitBranch size={48} class="text-muted-foreground/50" />
					<Clock size={20} class="absolute -bottom-1 -right-1 text-orange-500 bg-background rounded-full p-1" />
				</div>
			</div>
			
			<div class="space-y-2">
				<h4 class="font-medium text-foreground">Coming Soon</h4>
				<p class="text-sm text-muted-foreground leading-relaxed">
					Git integration and version control features will be available in a future update.
				</p>
			</div>
			
			<div class="flex items-center gap-2 text-xs text-muted-foreground bg-muted/50 rounded-lg p-3">
				<AlertCircle size={14} />
				<span>Stay tuned for Git support!</span>
			</div>
		</div>
	</div>
</div>