<script lang="ts">
  let { draft = $bindable(''), disabled = false, busy = false, generating = false, cancelling = false, onSend, onCancel }: {
    draft?: string; disabled?: boolean; busy?: boolean; generating?: boolean; cancelling?: boolean;
    onSend: () => void; onCancel: () => void;
  } = $props();
  let textarea: HTMLTextAreaElement | undefined = $state();

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!disabled && !busy && draft.trim()) onSend();
  }

  function keydown(event: KeyboardEvent) {
    if (event.key !== 'Enter' || event.shiftKey || event.ctrlKey || event.altKey || event.metaKey || event.isComposing || event.keyCode === 229) return;
    event.preventDefault();
    if (!event.repeat && !disabled && !busy && draft.trim()) onSend();
  }

  $effect(() => {
    void draft;
    if (textarea) {
      textarea.style.height = 'auto';
      textarea.style.height = Math.min(180, Math.max(48, textarea.scrollHeight)) + 'px';
    }
  });
</script>

<form class="composer" onsubmit={submit}>
  <label for="chat-draft">Mensagem</label>
  <textarea id="chat-draft" bind:this={textarea} bind:value={draft} disabled={disabled || busy} rows="2" onkeydown={keydown} aria-describedby="composer-help" placeholder="Converse com a Tina…" />
  <div class="actions">
    <small id="composer-help">Enter para enviar · Shift+Enter para nova linha</small>
    {#if busy}
      <button type="button" disabled={!generating || cancelling} onclick={onCancel}>{cancelling ? 'Interrompendo…' : 'Interromper'}</button>
    {:else}
      <button type="submit" disabled={disabled || !draft.trim()}>Enviar</button>
    {/if}
  </div>
</form>

<style>
  .composer { display: flex; flex-direction: column; gap: .5rem; }
  label { font-weight: 600; }
  textarea { min-height: 48px; max-height: 180px; overflow-y: auto; resize: none; padding: .75rem; border: 1px solid var(--tina-border-strong); border-radius: var(--tina-radius-md); font: inherit; background: var(--tina-surface); color: var(--tina-text); }
  .actions { display: flex; align-items: center; justify-content: space-between; gap: .5rem; }
  small { color: var(--tina-text-muted); }
  button { border: 1px solid var(--tina-border-strong); border-radius: var(--tina-radius-pill); padding: .55rem 1rem; font: inherit; font-weight: 600; color: var(--tina-pink-deep); background: var(--tina-surface); cursor: pointer; }
  button:disabled { opacity: .55; cursor: default; }
  button:focus-visible, textarea:focus-visible { outline: 2px solid var(--tina-pink); outline-offset: 2px; }
</style>
