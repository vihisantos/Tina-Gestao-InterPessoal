<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke, Channel } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import Badge from '$lib/components/Badge.svelte';
  import Button from '$lib/components/Button.svelte';
  import Card from '$lib/components/Card.svelte';
  import Logo from '$lib/components/Logo.svelte';

  type BootstrapInfo = {
    version: string;
    databasePath: string;
    databaseReady: boolean;
    message: string;
  };

  let info = $state<BootstrapInfo | null>(null);
  let error = $state<string | null>(null);
  let pingResult = $state<string | null>(null);
  let readyEvent = $state(false);

  type OwnerStatus = { configured: boolean; unlocked: boolean; displayName: string | null };

  let ownerStatus = $state<OwnerStatus | null>(null);
  let ownerName = $state('');
  let ownerPassword = $state('');
  let ownerConfirm = $state('');
  let ownerBusy = $state(false);
  let ownerError = $state<string | null>(null);

  type ChatSession = { id: string; title: string; created_at: string };
  type ChatMessage = { id: string; role: string; content: string; created_at: string };
  let chats = $state<ChatSession[]>([]);
  let selected = $state('');
  let messages = $state<ChatMessage[]>([]);
  let draft = $state('');
  let partial = $state('');
  let chatBusy = $state(false);
  let chatLoading = $state(false);
  let renameDraft = $state('');
  let renaming = $state(false);
  let chatError = $state('');
  let apiKey = $state('');
  let hasKey = $state(false);
  let chatEpoch = 0;

  async function loadChat() {
    if (chatBusy || chatLoading || !ownerStatus?.unlocked) return;
    const epoch = ++chatEpoch;
    chatLoading = true;
    try {
      const list = await invoke<ChatSession[]>('chat_list');
      const keyAvailable = await invoke<boolean>('chat_has_key');
      if (epoch !== chatEpoch || !ownerStatus?.unlocked) return;
      const id = list.find((chat) => chat.id === selected)?.id ?? list[0]?.id ?? '';
      const history = id ? await invoke<ChatMessage[]>('chat_history', { sessionId: id }) : [];
      if (epoch !== chatEpoch || !ownerStatus?.unlocked) return;
      chats = list;
      hasKey = keyAvailable;
      selected = id;
      messages = history;
      partial = '';
      chatError = '';
    } catch (cause) { if (epoch === chatEpoch) chatError = String(cause); }
    finally { if (epoch === chatEpoch) chatLoading = false; }
  }

  async function openChat(id: string) {
    if (chatBusy || chatLoading || !ownerStatus?.unlocked) return;
    const epoch = ++chatEpoch;
    chatLoading = true;
    try {
      const history = await invoke<ChatMessage[]>('chat_history', { sessionId: id });
      if (epoch !== chatEpoch || !ownerStatus?.unlocked) return;
      selected = id;
      messages = history;
      partial = '';
      draft = '';
      renaming = false;
      renameDraft = '';
      chatError = '';
    } catch (cause) { if (epoch === chatEpoch) chatError = String(cause); }
    finally { if (epoch === chatEpoch) chatLoading = false; }
  }

  async function renameChat(event: SubmitEvent) {
    event.preventDefault();
    if (chatBusy || chatLoading || !selected || !renameDraft.trim() || !ownerStatus?.unlocked) return;
    const epoch = chatEpoch;
    const id = selected;
    chatLoading = true;
    chatError = '';
    try {
      const updated = await invoke<ChatSession>('chat_rename', { sessionId: id, title: renameDraft });
      if (epoch !== chatEpoch || !ownerStatus?.unlocked) return;
      chats = chats.map((chat) => chat.id === id ? updated : chat);
      renaming = false;
      renameDraft = '';
    } catch (cause) { if (epoch === chatEpoch) chatError = String(cause); }
    finally { if (epoch === chatEpoch) chatLoading = false; }
  }

  async function saveKey(event: SubmitEvent) {
    event.preventDefault();
    try {
      await invoke('chat_set_key', { key: apiKey });
      hasKey = true;
      chatError = '';
    } catch (cause) { chatError = String(cause); }
    finally { apiKey = ''; }
  }

  async function sendChat(event: SubmitEvent) {
    event.preventDefault();
    if (chatBusy || chatLoading || !ownerStatus?.unlocked || !draft.trim() || !hasKey) return;
    chatBusy = true;
    chatError = '';
    partial = '';
    const epoch = chatEpoch;
    try {
      if (!selected) {
        const session = await invoke<ChatSession>('chat_create', { title: draft.trim().slice(0, 80) });
        if (epoch !== chatEpoch) return;
        selected = session.id;
        chats = [session, ...chats];
      }
      const sessionId = selected;
      const message = await invoke<ChatMessage>('chat_append', { sessionId, content: draft });
      if (epoch !== chatEpoch) return;
      messages = [...messages, message];
      draft = '';
      const onDelta = new Channel<string>();
      onDelta.onmessage = (delta) => { if (epoch === chatEpoch) partial += delta; };
      const answer = await invoke<ChatMessage>('chat_generate', { sessionId, onDelta });
      if (epoch === chatEpoch) messages = [...messages, answer];
    } catch (cause) { if (epoch === chatEpoch) chatError = String(cause); }
    finally { if (epoch === chatEpoch) { partial = ''; chatBusy = false; } }
  }

  async function cancelChat() {
    try { await invoke('chat_cancel'); }
    catch (cause) { chatError = String(cause); }
  }

  async function refreshOwner() {
    try {
      ownerStatus = await invoke<OwnerStatus>('owner_status');
      ownerError = null;
      if (ownerStatus.unlocked) await loadChat();
    } catch (cause) {
      ownerError = String(cause);
    }
  }

  async function submitOwner(event: SubmitEvent) {
    event.preventDefault();
    if (!ownerStatus || ownerBusy) return;
    if (!ownerStatus.configured && ownerPassword !== ownerConfirm) {
      ownerError = 'As senhas precisam ser iguais.';
      return;
    }
    ownerBusy = true;
    ownerError = null;
    try {
      if (!ownerStatus.configured) {
        await invoke('create_owner_account', { displayName: ownerName, password: ownerPassword });
      }
      await invoke('sign_in_owner', { password: ownerPassword });
      ownerPassword = '';
      ownerConfirm = '';
      await refreshOwner();
      await loadBootstrap();
    } catch (cause) {
      await refreshOwner();
      ownerError = String(cause);
    } finally {
      ownerBusy = false;
    }
  }

  async function lockOwner() {
    try {
      if (chatBusy) await cancelChat();
      await invoke('lock_owner');
      chatEpoch++;
      chats = []; messages = []; selected = ''; partial = ''; draft = ''; apiKey = ''; hasKey = false; chatBusy = false; chatLoading = false; renaming = false; renameDraft = ''; chatError = '';
      await refreshOwner();
      await loadBootstrap();
    } catch (cause) {
      ownerError = String(cause);
    }
  }

  async function loadBootstrap() {
    try {
      info = await invoke<BootstrapInfo>('bootstrap');
      error = null;
    } catch (cause) {
      error = String(cause);
    }
  }

  async function checkBridge() {
    try {
      pingResult = await invoke<string>('ping');
      error = null;
    } catch (cause) {
      error = String(cause);
    }
  }

  onMount(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    listen('tina://ready', () => {
      readyEvent = true;
      void loadBootstrap();
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });

    void loadBootstrap();
    void refreshOwner();

    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<svelte:head>
  <title>Tina — Inteligência Pessoal</title>
  <link rel="icon" href="/tina.png" />
</svelte:head>

<main>
  <header>
    <div class="brand">
      <Logo />
      <div>
        <p class="eyebrow">TINA · GESTÃO INTERPESSOAL</p>
        <h1>A sua operação, sob sua autoridade.</h1>
      </div>
    </div>
    <span class="pill">{info ? `v${info.version}` : 'conectando…'}</span>
  </header>

  {#if !ownerStatus?.unlocked}
  <section class="intro">
    <div class="intro-text">
      <p>
        Primeira versão executável da Tina. Ela abre em uma janela desktop real,
        mantém todos os dados na máquina e já conversa com o backend Rust por
        comandos e eventos nativos do Tauri.
      </p>
      {#if error}
        <p class="error" role="alert">Falha de comunicação: {error}</p>
      {/if}
    </div>
    <img class="mascot" src="/tina.png" alt="" aria-hidden="true" />
  </section>
  {/if}

  <section class="owner-onboarding" aria-label="Conta do proprietário">
    <Card title={ownerStatus?.configured ? 'Acesso do proprietário' : 'Crie sua conta na Tina'}>
      {#if ownerStatus === null}
        <p>Verificando conta local…</p>
      {:else if ownerStatus.unlocked}
        <p>Bem-vindo, {ownerStatus.displayName}. Sua sessão está desbloqueada.</p>
        <Button onclick={lockOwner}>Bloquear sessão</Button>
      {:else}
        <p>
          {ownerStatus.configured
            ? 'Digite sua senha mestra para entrar.'
            : 'Primeiro acesso: crie a conta do proprietário. A Tina gerará uma chave forte e a guardará no cofre do Windows.'}
        </p>
        <form class="owner-form" onsubmit={submitOwner}>
          {#if !ownerStatus.configured}
            <label for="owner-name">Seu nome</label>
            <input id="owner-name" autocomplete="name" minlength="2" maxlength="80" required bind:value={ownerName} />
          {/if}
          <label for="owner-password">Senha mestra</label>
          <input
            id="owner-password"
            type="password"
            autocomplete={ownerStatus.configured ? 'current-password' : 'new-password'}
            minlength={ownerStatus.configured ? undefined : 12}
            maxlength="256"
            required
            bind:value={ownerPassword}
          />
          {#if !ownerStatus.configured}
            <label for="owner-confirm">Confirme sua senha</label>
            <input
              id="owner-confirm"
              type="password"
              autocomplete="new-password"
              minlength="12"
              maxlength="256"
              required
              bind:value={ownerConfirm}
            />
          {/if}
          <button type="submit" disabled={ownerBusy}>
            {ownerBusy ? 'Verificando…' : ownerStatus.configured ? 'Entrar na Tina' : 'Criar conta e entrar'}
          </button>
        </form>
        {#if !ownerStatus.configured}
          <p class="owner-warning">Guarde sua senha. A recuperação e exportação da chave serão implementadas em etapa futura.</p>
        {/if}
      {/if}
      {#if ownerError}
        <p role="alert" class="error">{ownerError}</p>
      {/if}
    </Card>
  </section>

  {#if ownerStatus?.unlocked}
    <section class="chat-shell" aria-label="Chat da Tina">
      <aside>
        <h2>Conversas</h2>
        <button disabled={chatBusy || chatLoading} onclick={() => { chatEpoch++; selected = ''; messages = []; partial = ''; draft = ''; renaming = false; renameDraft = ''; }}>Nova conversa</button>
        {#each chats as chat (chat.id)}
          <button disabled={chatBusy || chatLoading} onclick={() => openChat(chat.id)} aria-pressed={selected === chat.id}>{chat.title}</button>
        {/each}
      </aside>
      <div class="chat-main">
        <h2>Tina</h2>
        {#if selected}
          {#if renaming}
            <form onsubmit={renameChat}>
              <label for="chat-title">Nome da conversa</label>
              <input id="chat-title" required maxlength="120" bind:value={renameDraft} disabled={chatLoading || chatBusy} />
              <button type="submit" disabled={chatLoading || chatBusy || !renameDraft.trim()}>Salvar nome</button>
              <button type="button" disabled={chatLoading} onclick={() => { renaming = false; renameDraft = ''; }}>Cancelar</button>
            </form>
          {:else}
            <button disabled={chatBusy || chatLoading} onclick={() => { renameDraft = chats.find((chat) => chat.id === selected)?.title ?? ''; renaming = true; }}>Renomear conversa</button>
          {/if}
        {/if}
        {#if !hasKey}
          <form onsubmit={saveKey}>
            <label for="nvidia-key">Chave NVIDIA NIM</label>
            <input id="nvidia-key" type="password" autocomplete="off" required bind:value={apiKey} />
            <button type="submit">Salvar no cofre Windows</button>
          </form>
        {/if}
        <div class="chat-history" role="log" aria-live="polite" aria-busy={chatLoading}>
          {#if !messages.length}<p>Olá! O que vamos fazer hoje?</p>{/if}
          {#each messages as message (message.id)}
            <article><strong>{message.role === 'user' ? 'Você' : 'Tina'}</strong><p>{message.content}</p></article>
          {/each}
          {#if partial}<article><strong>Tina</strong><p>{partial}</p></article>{/if}
        </div>
        {#if chatError}<p role="alert" class="error">{chatError}</p>{/if}
        <form onsubmit={sendChat}>
          <label for="chat-draft">Mensagem</label>
          <textarea id="chat-draft" bind:value={draft} disabled={chatBusy || chatLoading || !hasKey} rows="3"></textarea>
          <button type="submit" disabled={chatBusy || chatLoading || !hasKey || !draft.trim()}>Enviar</button>
          {#if chatBusy}<button type="button" onclick={cancelChat}>Interromper</button>{/if}
        </form>
      </div>
    </section>
  {:else}
  <section class="grid" aria-label="Estado da instalação">
    <Card title="Ambiente desktop">
      {#snippet badge()}
        <Badge tone="ok">Tauri 2</Badge>
      {/snippet}
      <p class="value">{info ? info.message : 'Lendo estado local…'}</p>
      <p class="detail">
        Evento <code>tina://ready</code>
        <strong class:ok-text={readyEvent}>{readyEvent ? 'recebido' : 'aguardando'}</strong>
      </p>
    </Card>

    <Card title="Banco local">
      {#snippet badge()}
        <Badge tone={info?.databaseReady ? 'ok' : 'default'}>
          {info?.databaseReady ? 'pronto' : '…'}
        </Badge>
      {/snippet}
      <p class="value path">{info?.databasePath ?? 'Localizando %APPDATA%\\tina\\tina.db…'}</p>
      <p class="detail">SQLite criado e migrado na inicialização, sem serviço externo.</p>
    </Card>

    <Card title="Ponte Rust ↔ Svelte">
      {#snippet badge()}
        <Badge tone={pingResult === 'pong' ? 'ok' : 'default'}>
          {pingResult === 'pong' ? 'ativa' : 'testar'}
        </Badge>
      {/snippet}
      <p class="value">{pingResult ? `invoke('ping') → ${pingResult}` : 'Nenhum teste enviado ainda.'}</p>
      <Button onclick={checkBridge}>Verificar comunicação</Button>
    </Card>
  </section>

  <footer>
    <p>
      Altere qualquer texto ou cor deste arquivo e salve: a janela recarrega
      automaticamente pelo hot reload do SvelteKit. Alterações em Rust
      recompilam o aplicativo via Tauri.
    </p>
  </footer>
  {/if}
</main>

<style>
  .chat-shell { display: grid; grid-template-columns: minmax(130px, 220px) 1fr; gap: 1rem; }
  .chat-shell aside, .chat-main { display: flex; flex-direction: column; gap: .75rem; min-width: 0; padding: 1rem; border: 1px solid var(--tina-border); border-radius: 12px; background: var(--tina-surface-tint); }
  .chat-history { min-height: 220px; max-height: 55vh; overflow: auto; }
  .chat-history p { white-space: pre-wrap; overflow-wrap: anywhere; }
  .chat-main form { display: flex; flex-direction: column; gap: .5rem; }
  .chat-shell button, .chat-shell input, .chat-shell textarea { padding: .65rem; border: 1px solid var(--tina-border); border-radius: 8px; font: inherit; }
  @media (max-width: 640px) { .chat-shell { grid-template-columns: 1fr; } }

  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    background-color: var(--tina-bg);
    background-image: var(--tina-gradient-page);
    color: var(--tina-text);
    font-family: var(--tina-font);
    -webkit-font-smoothing: antialiased;
  }

  main {
    max-width: 1060px;
    margin: 0 auto;
    padding: 2.5rem 2rem 3rem;
  }

  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .eyebrow {
    margin: 0 0 0.3rem;
    color: var(--tina-text-pink-soft);
    font-size: 0.72rem;
    font-weight: 600;
    letter-spacing: 0.14em;
  }

  h1 {
    margin: 0;
    font-size: 1.8rem;
    line-height: 1.2;
    color: var(--tina-title);
    letter-spacing: -0.01em;
  }

  .pill {
    padding: 0.4rem 0.85rem;
    border: 1px solid #ffd6e4;
    border-radius: var(--tina-radius-pill);
    background: var(--tina-surface-tint);
    color: var(--tina-text-pink);
    font-size: 0.8rem;
    font-weight: 600;
    white-space: nowrap;
  }

  .intro {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    padding: 1.4rem 1.3rem 1.4rem 1.4rem;
    margin-bottom: 1.25rem;
    border: 1px solid var(--tina-border);
    border-left: 3px solid var(--tina-pink-soft);
    border-radius: var(--tina-radius-lg);
    background: var(--tina-gradient-soft);
  }

  .intro-text {
    flex: 1 1 auto;
  }

  .intro p {
    margin: 0;
    line-height: 1.65;
    color: var(--tina-text-muted);
  }

  .error {
    margin-top: 0.75rem !important;
    color: var(--tina-pink-deep) !important;
    font-weight: 600;
  }

  .mascot {
    flex: none;
    width: 7.5rem;
    height: auto;
    display: block;
    filter: drop-shadow(0 14px 18px rgba(208, 115, 146, 0.28));
    animation: tina-float 6s ease-in-out infinite;
  }

  @keyframes tina-float {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-7px);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .mascot {
      animation: none;
    }
  }

  .owner-onboarding {
    margin-bottom: 1.25rem;
  }

  .owner-onboarding p {
    color: var(--tina-text-muted);
  }

  .owner-form {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    max-width: 420px;
  }

  .owner-form label {
    font-weight: 600;
  }

  .owner-form input {
    padding: 0.75rem;
    border: 1px solid var(--tina-border);
    border-radius: 8px;
    background: white;
    color: var(--tina-text);
  }

  .owner-form button {
    padding: 0.75rem;
    border: none;
    border-radius: 8px;
    background: #ee6495;
    color: white;
    cursor: pointer;
    font-weight: 700;
  }

  .owner-form button:disabled {
    opacity: 0.6;
  }

  .owner-warning {
    font-size: 0.85rem;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 1rem;
  }

  .value {
    margin: 0;
    font-size: 1.05rem;
    line-height: 1.4;
    color: var(--tina-text);
  }

  .path {
    font-family: var(--tina-font-mono);
    font-size: 0.82rem;
    word-break: break-all;
    color: #c2416f;
  }

  .detail {
    margin: 0;
    color: var(--tina-text-muted);
    font-size: 0.85rem;
    line-height: 1.55;
  }

  .detail strong {
    color: #a9788a;
    font-weight: 600;
  }

  .detail strong.ok-text {
    color: var(--tina-pink-deep);
  }

  code {
    background: var(--tina-surface-tint);
    color: var(--tina-text-pink);
    padding: 0.12rem 0.35rem;
    border-radius: var(--tina-radius-sm);
    font-size: 0.85em;
  }

  footer {
    margin-top: 2rem;
    padding-top: 1rem;
    border-top: 1px solid #ffe8f0;
    color: var(--tina-text-soft);
    font-size: 0.82rem;
    line-height: 1.6;
  }

  @media (max-width: 640px) {
    .mascot {
      width: 5.5rem;
    }
  }
</style>
