<script lang="ts">
  import MarkdownContent from './MarkdownContent.svelte';

  let {
    role,
    content,
    streaming = false
  }: {
    role: string;
    content: string;
    streaming?: boolean;
  } = $props();

  const label = $derived(role === 'user' ? 'Você' : 'Tina');
  let copyStatus = $state('');

  async function copyResponse() {
    try {
      await navigator.clipboard.writeText(content);
      copyStatus = 'Resposta copiada.';
    } catch {
      copyStatus = 'Não foi possível copiar a resposta.';
    }
  }
</script>

<article class="chat-message" data-role={role === 'user' ? 'user' : 'assistant'}>
  <strong>{label}</strong>
  <MarkdownContent {content} {streaming} />
  {#if role === 'assistant' && !streaming && content}
    <div class="response-actions"><button type="button" onclick={copyResponse}>Copiar resposta</button><span role="status">{copyStatus}</span></div>
  {/if}
</article>

<style>
  .chat-message {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.75rem;
    border: 1px solid var(--tina-border);
    border-radius: var(--tina-radius-md);
    background: var(--tina-surface);
  }

  .chat-message[data-role='user'] {
    border-color: var(--tina-border-strong);
    background: var(--tina-surface-tint);
  }

  .response-actions { display: flex; align-items: center; gap: .5rem; font-size: .75rem; color: var(--tina-text-muted); }
  .response-actions button { border: 1px solid var(--tina-border); border-radius: var(--tina-radius-sm); padding: .25rem .5rem; background: var(--tina-surface); color: var(--tina-pink-deep); font: inherit; cursor: pointer; }
  .response-actions button:focus-visible { outline: 2px solid var(--tina-pink); }
  strong {
    color: var(--tina-pink-deep);
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
</style>
