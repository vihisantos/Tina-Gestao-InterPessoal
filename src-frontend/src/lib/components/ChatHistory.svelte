<script lang="ts">
  import { onMount } from 'svelte';
  import ChatMessage from './ChatMessage.svelte';
  import { isNearBottom } from '../chat/scroll';

  type Message = {
    id: string;
    role: string;
    content: string;
    tool_call_id?: string | null;
    tool_calls_json?: string | null;
  };
  let { messages, partial = '', loading = false, generating = false, cancelling = false, greeting = 'Olá! O que vamos fazer hoje?' }: {
    messages: Message[]; partial?: string; loading?: boolean; generating?: boolean; cancelling?: boolean; greeting?: string;
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
      {#if !messages.length && !partial && !generating}
        <div class="greeting" role="status">
          <p class="greeting-name">Tina</p>
          <p>{greeting}</p>
        </div>
      {/if}
      {#each messages as message (message.id)}
        {#if message.tool_calls_json}
          <div class="tool-note" role="note">A Tina solicitou ações com ferramentas. Revise o plano antes de autorizar.</div>
        {:else if message.tool_call_id}
          <div class="tool-note" role="note">Resultado de ferramenta registrado no plano.</div>
        {:else}
          <ChatMessage role={message.role} content={message.content} />
        {/if}
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
  .tool-note { align-self: flex-start; max-width: 42rem; padding: .45rem .7rem; border: 1px dashed var(--tina-border-strong); border-radius: var(--tina-radius-md); background: var(--tina-surface-tint); color: var(--tina-text-muted); font-size: .8rem; }
  .history-shell { position: relative; min-width: 0; }
  .chat-history { min-height: 220px; max-height: 55vh; overflow: auto; overflow-anchor: none; }
  .history-content { display: flex; flex-direction: column; gap: .75rem; min-width: 0; }
  .greeting { padding: 1rem 1.1rem; border: 1px solid var(--tina-border); border-radius: 12px; background: linear-gradient(180deg, #fff7fa, #fff); color: var(--tina-text); max-width: 42rem; }
  .greeting p { margin: 0; line-height: 1.55; }
  .greeting-name { font-size: .78rem; font-weight: 700; letter-spacing: .04em; text-transform: uppercase; color: var(--tina-pink-deep); margin-bottom: .35rem !important; }
  .latest { display: block; margin: .35rem auto 0; padding: .4rem .75rem; color: var(--tina-pink-deep); background: var(--tina-surface); border: 1px solid var(--tina-border-strong); border-radius: var(--tina-radius-pill); font: inherit; cursor: pointer; }
  .latest:focus-visible, .chat-history:focus-visible { outline: 2px solid var(--tina-pink); outline-offset: 2px; }
</style>
