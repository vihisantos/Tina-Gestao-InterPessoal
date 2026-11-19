<script lang="ts">
  import type { Snippet } from 'svelte';

  let { title, badge, children }: { title: string; badge?: Snippet; children: Snippet } = $props();
</script>

<article class="card">
  <header class="card-top">
    <h2>{title}</h2>
    {#if badge}{@render badge()}{/if}
  </header>
  <div class="card-body">{@render children()}</div>
</article>

<style>
  .card {
    background: var(--tina-surface);
    border: 1px solid var(--tina-border);
    border-radius: var(--tina-radius-lg);
    padding: 1.35rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-height: 190px;
    box-shadow: var(--tina-shadow-card);
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .card:hover {
    border-color: var(--tina-border-strong);
    box-shadow: var(--tina-shadow-card-hover);
  }

  .card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  h2 {
    margin: 0;
    font-size: 0.95rem;
    color: #8d6b78;
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  /* O corpo não cria caixa: os filhos entram direto no flex do card. */
  .card-body {
    display: contents;
  }
</style>
