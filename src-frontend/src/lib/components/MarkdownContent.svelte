<script lang="ts">
  import { mount, unmount } from 'svelte';
  import MermaidDiagram from './MermaidDiagram.svelte';
  import { renderMarkdown } from '../chat/markdown';

  let { content = '', streaming = false }: { content?: string; streaming?: boolean } = $props();

  let container: HTMLDivElement | undefined = $state();
  let diagrams: Array<{ target: Element; instance: Record<string, unknown> }> = [];

  const rendered = $derived(renderMarkdown(content, { renderDiagrams: !streaming }));

  function unmountDiagrams() {
    for (const diagram of diagrams) void unmount(diagram.instance);
    diagrams = [];
  }

  function mountDiagrams() {
    if (!container || typeof window === 'undefined') return;
    unmountDiagrams();
    for (const placeholder of Array.from(container.querySelectorAll<HTMLElement>('[data-mermaid-index]'))) {
      const code = rendered.diagrams[Number(placeholder.dataset.mermaidIndex)];
      if (code === undefined) continue;
      const instance = mount(MermaidDiagram, { target: placeholder, props: { code } });
      diagrams.push({ target: placeholder, instance });
    }
  }

  $effect(() => {
    void rendered.html;
    mountDiagrams();
    return () => unmountDiagrams();
  });

  async function handleCopy(event: Event) {
    const target = event.target instanceof Element ? event.target : null;
    const button = target?.closest<HTMLButtonElement>('[data-copy-code]');
    if (!button || !container?.contains(button)) return;

    const code = button.parentElement?.querySelector('code')?.textContent ?? '';
    try {
      await navigator.clipboard.writeText(code);
      button.textContent = 'Copiado!';
      button.setAttribute('aria-label', 'Código copiado');
    } catch {
      button.textContent = 'Falhou';
      button.setAttribute('aria-label', 'Não foi possível copiar o código');
    }

    window.setTimeout(() => {
      if (!button.isConnected) return;
      button.textContent = 'Copiar';
      button.setAttribute('aria-label', 'Copiar código');
    }, 1500);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    const target = event.target instanceof Element ? event.target : null;
    if (!target?.closest('[data-copy-code]')) return;
    event.preventDefault();
    void handleCopy(event);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="markdown"
  role="group"
  aria-label="Conteúdo da mensagem"
  bind:this={container}
  onclick={handleCopy}
  onkeydown={handleKeydown}
>
  {@html rendered.html}
</div>

<style>
  .markdown {
    min-width: 0;
    color: var(--tina-text);
    font-size: 0.95rem;
    line-height: 1.65;
    overflow-wrap: anywhere;
  }

  :global(.markdown > :first-child) {
    margin-top: 0;
  }

  :global(.markdown > :last-child) {
    margin-bottom: 0;
  }

  :global(.markdown h1),
  :global(.markdown h2),
  :global(.markdown h3),
  :global(.markdown h4),
  :global(.markdown h5),
  :global(.markdown h6) {
    margin: 1rem 0 0.5rem;
    color: var(--tina-title);
    line-height: 1.25;
  }

  :global(.markdown h1) { font-size: 1.3rem; }
  :global(.markdown h2) { font-size: 1.15rem; }
  :global(.markdown h3) { font-size: 1.05rem; }

  :global(.markdown p),
  :global(.markdown ul),
  :global(.markdown ol) {
    margin: 0.5rem 0;
  }

  :global(.markdown ul),
  :global(.markdown ol) {
    padding-left: 1.5rem;
  }

  :global(.markdown li + li) {
    margin-top: 0.25rem;
  }

  :global(.markdown blockquote) {
    margin: 0.75rem 0;
    padding: 0.25rem 0.75rem;
    border-left: 3px solid var(--tina-pink-soft);
    border-radius: var(--tina-radius-sm);
    background: var(--tina-surface-tint);
    color: var(--tina-text-muted);
  }

  :global(.markdown blockquote p) {
    margin: 0.25rem 0;
  }

  :global(.markdown a.tina-safe-link) {
    color: var(--tina-text-pink);
    text-decoration-color: var(--tina-pink-soft);
    text-underline-offset: 0.15em;
  }

  :global(.markdown a.tina-safe-link:hover) {
    color: var(--tina-pink-deep);
  }

  :global(.markdown a.tina-blocked-link) {
    color: var(--tina-text-muted);
    text-decoration: line-through;
    cursor: not-allowed;
  }

  :global(.markdown code) {
    padding: 0.12rem 0.35rem;
    border-radius: var(--tina-radius-sm);
    background: var(--tina-surface-tint);
    color: var(--tina-text-pink);
    font-family: var(--tina-font-mono);
    font-size: 0.85em;
  }

  :global(.markdown .tina-code-wrap) {
    position: relative;
    margin: 0.75rem 0;
  }

  :global(.markdown .tina-copy-button) {
    position: absolute;
    top: 0.35rem;
    right: 0.35rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--tina-border-strong);
    border-radius: var(--tina-radius-sm);
    background: var(--tina-surface);
    color: var(--tina-pink-deep);
    cursor: pointer;
    font-family: var(--tina-font);
    font-size: 0.72rem;
    font-weight: 600;
  }

  :global(.markdown .tina-copy-button:hover) {
    background: var(--tina-surface-pink);
  }

  :global(.markdown .tina-copy-button:focus-visible) {
    outline: 2px solid var(--tina-pink);
    outline-offset: 2px;
  }

  :global(.markdown pre.tina-code-block) {
    margin: 0;
    padding: 0.75rem;
    padding-right: 4.5rem;
    overflow-x: auto;
    border: 1px solid var(--tina-border);
    border-radius: var(--tina-radius-md);
    background: var(--tina-surface-tint);
  }

  :global(.markdown pre.tina-code-block code) {
    padding: 0;
    background: transparent;
    color: var(--tina-text);
    font-size: 0.82rem;
    white-space: pre;
  }

  :global(.markdown table) {
    width: 100%;
    margin: 0.75rem 0;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  :global(.markdown th),
  :global(.markdown td) {
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--tina-border);
    text-align: left;
  }

  :global(.markdown th) {
    background: var(--tina-surface-pink);
    color: var(--tina-pink-deep);
    font-weight: 600;
  }

  :global(.markdown hr) {
    margin: 1rem 0;
    border: 0;
    border-top: 1px solid var(--tina-border);
  }
</style>
