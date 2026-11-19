<script lang="ts">
  import type { Snippet } from 'svelte';

  let { tone = 'default', children }: { tone?: 'default' | 'ok'; children: Snippet } = $props();
</script>

<span class="badge" class:ok={tone === 'ok'}>{@render children()}</span>

<style>
  .badge {
    padding: 0.2rem 0.6rem;
    border-radius: var(--tina-radius-pill);
    background: var(--tina-surface-tint);
    border: 1px solid var(--tina-border);
    color: #a9788a;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    white-space: nowrap;
  }

  .badge.ok {
    background: var(--tina-surface-pink);
    border-color: var(--tina-border-strong);
    color: var(--tina-pink-deep);
  }
</style>
