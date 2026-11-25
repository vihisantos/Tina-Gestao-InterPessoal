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
</script>

<article class="chat-message" data-role={role === 'user' ? 'user' : 'assistant'}>
  <strong>{label}</strong>
  <MarkdownContent {content} {streaming} />
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

  strong {
    color: var(--tina-pink-deep);
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
</style>
