<script module lang="ts">
  let diagramCounter = 0;
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import mermaid from 'mermaid';
  import { sanitizeMermaidSvg } from '../chat/markdown';

  let { code = '' }: { code?: string } = $props();

  const diagramId = `tina-mermaid-${++diagramCounter}`;

  let svg = $state('');
  let failed = $state(false);
  let disposed = false;

  mermaid.initialize({
    startOnLoad: false,
    securityLevel: 'strict',
    theme: 'base',
    themeVariables: {
      fontFamily: 'var(--tina-font)'
    }
  });

  async function renderDiagram() {
    try {
      const result = await mermaid.render(diagramId, code);
      if (!disposed) {
        svg = sanitizeMermaidSvg(result.svg);
        failed = false;
      }
    } catch {
      if (!disposed) {
        failed = true;
        svg = '';
      }
    }
  }

  onMount(() => {
    disposed = false;
    void renderDiagram();
    return () => {
      disposed = true;
    };
  });
</script>

{#if failed}
  <div class="tina-mermaid-fallback" role="note">
    <p>Não foi possível gerar o diagrama. Código original:</p>
    <pre><code>{code}</code></pre>
  </div>
{:else if svg}
  <div class="tina-mermaid" aria-label="Diagrama Mermaid">{@html svg}</div>
{:else}
  <p class="tina-mermaid-loading">Gerando diagrama…</p>
{/if}

<style>
  .tina-mermaid-fallback,
  .tina-mermaid {
    margin: 0.75rem 0;
    padding: 0.75rem;
    border: 1px solid var(--tina-border);
    border-radius: var(--tina-radius-md);
    background: var(--tina-surface-tint);
    color: var(--tina-text-muted);
    font-size: 0.85rem;
    line-height: 1.55;
  }

  .tina-mermaid-fallback p {
    margin: 0 0 0.5rem;
    color: var(--tina-pink-deep);
    font-weight: 600;
  }

  .tina-mermaid-fallback pre {
    margin: 0;
    padding: 0.5rem;
    overflow-x: auto;
    border-radius: var(--tina-radius-sm);
    background: var(--tina-surface);
    color: var(--tina-text);
    font-family: var(--tina-font-mono);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .tina-mermaid-loading {
    margin: 0.75rem 0;
    color: var(--tina-text-muted);
    font-size: 0.85rem;
  }

  :global(.tina-mermaid svg) {
    display: block;
    width: 100%;
    height: auto;
    max-width: 100%;
    font-family: var(--tina-font);
  }

  :global(.tina-mermaid svg .node rect),
  :global(.tina-mermaid svg .node polygon),
  :global(.tina-mermaid svg .node path),
  :global(.tina-mermaid svg .cluster rect) {
    fill: var(--tina-surface-pink) !important;
    stroke: var(--tina-border-strong) !important;
  }

  :global(.tina-mermaid svg .node text),
  :global(.tina-mermaid svg .cluster text),
  :global(.tina-mermaid svg .edgeLabel) {
    fill: var(--tina-text) !important;
    color: var(--tina-text);
  }

  :global(.tina-mermaid svg .edgePath .path),
  :global(.tina-mermaid svg .arrowheadPath) {
    stroke: var(--tina-pink-strong) !important;
    fill: var(--tina-pink-strong) !important;
  }
</style>
