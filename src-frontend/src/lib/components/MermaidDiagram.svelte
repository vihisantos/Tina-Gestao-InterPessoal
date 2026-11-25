<script module lang="ts">
  let diagramCounter = 0;
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import mermaid from 'mermaid';
  import { isSafeMermaidSource, sanitizeMermaidSvg } from '../chat/markdown';

  let { code = '' }: { code?: string } = $props();

  const diagramId = `tina-mermaid-${++diagramCounter}`;

  let svg = $state('');
  let failed = $state(false);
  let disposed = false;

  function initializeMermaid() {
    const style = getComputedStyle(document.documentElement);
    const color = (token: string) => style.getPropertyValue(token).trim();
    const primary = color('--tina-surface-pink');
    const border = color('--tina-border-strong');
    const text = color('--tina-text');
    const line = color('--tina-pink-strong');
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: 'strict',
      htmlLabels: false,
      flowchart: { htmlLabels: false },
      maxTextSize: 20_000,
      maxEdges: 250,
      theme: 'base',
      themeVariables: {
        fontFamily: 'Segoe UI, sans-serif',
        ...(primary ? { primaryColor: primary } : {}),
        ...(border ? { primaryBorderColor: border } : {}),
        ...(text ? { primaryTextColor: text } : {}),
        ...(line ? { lineColor: line } : {})
      }
    });
  }

  async function renderDiagram() {
    try {
      if (!isSafeMermaidSource(code)) throw new Error('Diagrama fora do formato permitido');
      initializeMermaid();
      const result = await mermaid.render(diagramId, code);
      if (!disposed) {
        const clean = sanitizeMermaidSvg(result.svg);
        if (!clean.trim().startsWith('<svg')) throw new Error('SVG inválido');
        // Render as an inert image, never insert diagram markup into the app DOM.
        svg = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(clean);
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
  <div class="tina-mermaid"><img src={svg} alt="Diagrama Mermaid" /><details><summary>Ver código do diagrama</summary><pre><code>{code}</code></pre></details></div>
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

  .tina-mermaid img { display: block; width: 100%; height: auto; max-width: 100%; }
  .tina-mermaid details { margin-top: .5rem; }
  .tina-mermaid pre { overflow-x: auto; white-space: pre-wrap; overflow-wrap: anywhere; }
</style>
