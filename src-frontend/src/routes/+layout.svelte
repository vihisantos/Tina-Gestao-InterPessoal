<script lang="ts">
  import type { Snippet } from 'svelte';
  import '../theme.css';

  let { children }: { children: Snippet } = $props();
</script>

{@render children()}
