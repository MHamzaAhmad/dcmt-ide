<script lang="ts">
  import type { HTMLTextareaAttributes } from 'svelte/elements';
  import { cn, type WithElementRef } from '$lib/utils.js';

  type Props = WithElementRef<
    HTMLTextareaAttributes & {
      autoResize?: boolean;
      maxRows?: number;
    }
  >;

  let {
    ref = $bindable<HTMLTextAreaElement | null>(null),
    value = $bindable<string>(''),
    class: className,
    autoResize = true,
    maxRows = 3,
    ...restProps
  }: Props = $props();

  function resize() {
    if (!ref || !autoResize) return;
    // Reset height to measure scrollHeight accurately
    ref.style.height = 'auto';

    const style = getComputedStyle(ref);
    const lineHeight = parseFloat(style.lineHeight) || 20;
    const paddingTop = parseFloat(style.paddingTop) || 0;
    const paddingBottom = parseFloat(style.paddingBottom) || 0;
    const borderTop = parseFloat(style.borderTopWidth) || 0;
    const borderBottom = parseFloat(style.borderBottomWidth) || 0;

    const maxHeight = maxRows > 0
      ? lineHeight * maxRows + paddingTop + paddingBottom + borderTop + borderBottom
      : Number.POSITIVE_INFINITY;

    const newHeight = Math.min(ref.scrollHeight, maxHeight);
    ref.style.height = `${newHeight}px`;
    ref.style.overflowY = ref.scrollHeight > maxHeight ? 'auto' : 'hidden';
  }

  // Recalculate on value changes
  $effect(() => {
    // Trigger resize after DOM updates
    queueMicrotask(resize);
  });
</script>

<textarea
  bind:this={ref}
  data-slot="textarea"
  class={cn(
    // Base styles mirror Input component for consistency
    'border-input bg-background selection:bg-primary dark:bg-input/30 selection:text-primary-foreground ring-offset-background placeholder:text-muted-foreground shadow-xs w-full min-w-0 rounded-md border px-3 py-2 text-base outline-none transition-[color,box-shadow] disabled:cursor-not-allowed disabled:opacity-50 md:text-sm',
    'focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]',
    'aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive',
    'resize-none min-h-9',
    className
  )}
  rows={1}
  bind:value
  oninput={resize}
  {...restProps}
></textarea>
