<script lang="ts">
  import { onMount } from 'svelte';
  import ChatMessage from './ChatMessage.svelte';
  import { isNearBottom } from '../chat/scroll';

  type Message = { id: string; role: string; content: string };
  let { messages, partial = '', loading = false, generating = false, cancelling = false }: {
    messages: Message[]; partial?: string; loading?: boolean; generating?: boolean; cancelling?: boolean;
  } = $props();

  let viewport: HTMLDivElement;
  let content: HTMLDivElement;
  let following = $state(true);

  function jumpToLatest() {
    following = true;
    viewport.scrollTop = viewport.scrollHeight;
  }

  function onScroll() {
    following = isNearBottom(viewport);
  }

  onMount(() => {
    jumpToLatest();
    // Observe content, including diagrams that finish rendering asynchronously,
    // and viewport resizing. Never move a reader who has scrolled upward.
    const observer = new ResizeObserver(() => {
      if (following) jumpToLatest();
    });
    observer.observe(content);
    observer.observe(viewport);
    return () => observer.disconnect();
  });
</script>

<div class="history-shell">
  <!-- svelte-ignore a11y_no_noninteractive_tabindex (histórico rolável acessível por teclado) -->
  <div class="chat-history" role="log" aria-label="Histórico da conversa" aria-live="polite" aria-busy={loading} bind:this={viewport} onscroll={onScroll} tabindex="0">
    <div class="history-content" bind:this={content}>
      {#if !messages.length}<p>Olá! O que vamos fazer hoje?</p>{/if}
      {#each messages as message (message.id)}
        <ChatMessage role={message.role} content={message.content} />
      {/each}
      {#if partial}<ChatMessage role="assistant" content={partial} streaming />{/if}
      {#if generating}<p class="generation" role="status">{cancelling ? 'Interrompendo resposta…' : partial ? 'Tina está respondendo…' : 'Tina está preparando a resposta…'}</p>{/if}
    </div>
  </div>
  {#if !following}
    <button class="latest" type="button" onclick={jumpToLatest}>Ir para a última mensagem ↓</button>
  {/if}
</div>

<style>
  .generation { color: var(--tina-text-muted); font-size: .85rem; margin: .25rem 0; }
  .history-shell { position: relative; min-width: 0; }
  .chat-history { min-height: 220px; max-height: 55vh; overflow: auto; overflow-anchor: none; }
  .history-content { display: flex; flex-direction: column; gap: .75rem; min-width: 0; }
  .latest { display: block; margin: .35rem auto 0; padding: .4rem .75rem; color: var(--tina-pink-deep); background: var(--tina-surface); border: 1px solid var(--tina-border-strong); border-radius: var(--tina-radius-pill); font: inherit; cursor: pointer; }
  .latest:focus-visible, .chat-history:focus-visible { outline: 2px solid var(--tina-pink); outline-offset: 2px; }
</style>
