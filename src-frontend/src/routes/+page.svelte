<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke, Channel } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import Badge from '$lib/components/Badge.svelte';
  import Button from '$lib/components/Button.svelte';
  import Card from '$lib/components/Card.svelte';
  import ChatHistory from '$lib/components/ChatHistory.svelte';
  import ChatComposer from '$lib/components/ChatComposer.svelte';
  import Logo from '$lib/components/Logo.svelte';
  import PersonnelDirectory from '$lib/components/PersonnelDirectory.svelte';
  import DepartmentProposal from '$lib/components/DepartmentProposal.svelte';
  import OrganizationDirectory from '$lib/components/OrganizationDirectory.svelte';
  import ToolsDirectory from '$lib/components/ToolsDirectory.svelte';

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
  type ChatLimits = { maxUserMessageBytes: number };
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
  let ownerEpoch = 0;
  let sessionCheckRunning = false;
  let generating = $state(false);
  let cancelling = $state(false);
  let retryMessageId = $state('');
  let runId = '';
  let pendingSend: { sessionId: string; messageId: string; content: string; title: string; creating: boolean } | null = $state(null);
  let retryAvailable = $derived(!!retryMessageId && messages.at(-1)?.id === retryMessageId && messages.at(-1)?.role === 'user');
  let chatLimits = $state<ChatLimits>({ maxUserMessageBytes: 4032 });
  let view = $state<'chat' | 'pessoal' | 'organizations' | 'tools'>('chat');
  let initialGreeting = $derived.by(() => {
    const name = ownerStatus?.displayName?.trim();
    return name
      ? `Olá, ${name}. Eu sou a Tina. Como posso ajudar você hoje?`
      : 'Olá! Eu sou a Tina. Como posso ajudar você hoje?';
  });

  async function loadChat() {
    if (chatBusy || chatLoading || !ownerStatus?.unlocked) return;
    const epoch = ++chatEpoch;
    chatLoading = true;
    try {
      const [list, keyAvailable, limits] = await Promise.all([
        invoke<ChatSession[]>('chat_list'),
        invoke<boolean>('chat_has_key'),
        invoke<ChatLimits>('chat_limits')
      ]);
      if (epoch !== chatEpoch || !ownerStatus?.unlocked) return;
      const id = list.find((chat) => chat.id === selected)?.id ?? list[0]?.id ?? '';
      const history = id ? await invoke<ChatMessage[]>('chat_history', { sessionId: id }) : [];
      if (epoch !== chatEpoch || !ownerStatus?.unlocked) return;
      chats = list;
      hasKey = keyAvailable;
      chatLimits = limits;
      selected = id;
      messages = history;
      partial = '';
      retryMessageId = '';
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
      retryMessageId = '';
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
    if (chatBusy || chatLoading || !ownerStatus?.unlocked || !apiKey.trim()) return;
    const epoch = chatEpoch;
    const key = apiKey;
    apiKey = '';
    chatLoading = true;
    try {
      await invoke('chat_set_key', { key });
      if (epoch !== chatEpoch || !ownerStatus?.unlocked) return;
      hasKey = true;
      chatError = '';
    } catch (cause) { if (epoch === chatEpoch) chatError = String(cause); }
    finally { if (epoch === chatEpoch) chatLoading = false; }
  }

  async function generateAnswer(sessionId: string, userId: string, epoch: number) {
    const id = crypto.randomUUID();
    runId = id;
    retryMessageId = '';
    const onDelta = new Channel<string>();
    onDelta.onmessage = (delta) => {
      if (epoch === chatEpoch && generating && !cancelling) partial += delta;
    };
    try {
      await invoke('chat_prepare', { runId: id });
      if (epoch !== chatEpoch) return;
      generating = true;
      const answer = await invoke<ChatMessage>('chat_generate', { sessionId, runId: id, onDelta });
      if (epoch === chatEpoch) {
        messages = [...messages.filter((m) => m.id !== answer.id), answer];
        chatError = '';
      }
    } catch (cause) {
      if (epoch !== chatEpoch) return;
      chatError = cancelling ? 'Resposta interrompida.' : String(cause);
      // Reconcile persisted history before offering retry. IPC failure could
      // have occurred after a successful write; never append the user again.
      try {
        const history = await invoke<ChatMessage[]>('chat_history', { sessionId });
        if (epoch !== chatEpoch) return;
        messages = history;
        const last = history.at(-1);
        if (last?.id === userId && last.role === 'user') retryMessageId = userId;
        else if (last?.role === 'assistant') chatError = '';
      } catch {
        if (epoch === chatEpoch) chatError += ' Não foi possível confirmar o histórico. Reabra a conversa antes de tentar novamente.';
      }
    } finally {
      await invoke('chat_cancel', { runId: id }).catch(() => {});
      if (epoch === chatEpoch) { runId = ''; generating = false; cancelling = false; partial = ''; }
    }
  }

  async function sendChat() {
    if (chatBusy || chatLoading || !ownerStatus?.unlocked || !draft.trim() || !hasKey) return;
    chatBusy = true;
    chatError = '';
    retryMessageId = '';
    partial = '';
    const epoch = chatEpoch;
    const content = draft;
    const maxBytes = chatLimits.maxUserMessageBytes;
    if (new TextEncoder().encode(content).length > maxBytes) {
      chatError = `Mensagem extensa demais. Divida o texto em partes de até ${maxBytes.toLocaleString('pt-BR')} bytes.`;
      chatBusy = false;
      return;
    }
    const attempt = pendingSend ?? {
      sessionId: selected || crypto.randomUUID(),
      messageId: crypto.randomUUID(),
      content,
      title: Array.from(content.trim()).slice(0, 80).join(''),
      creating: !selected
    };
    if (attempt.content !== content) {
      chatError = 'Confirme primeiro o envio anterior sem alterar o texto.';
      chatBusy = false;
      return;
    }
    pendingSend = attempt;
    try {
      if (attempt.creating) {
        const session = await invoke<ChatSession>('chat_create', { title: attempt.title, requestId: attempt.sessionId });
        if (epoch !== chatEpoch) return;
        selected = session.id;
        chats = [session, ...chats.filter((c) => c.id !== session.id)];
        pendingSend = { ...attempt, creating: false };
      }
      const sessionId = attempt.sessionId;
      const message = await invoke<ChatMessage>('chat_append', { sessionId, content, requestId: attempt.messageId });
      if (epoch !== chatEpoch) return;
      messages = [...messages.filter((m) => m.id !== message.id), message];
      pendingSend = null;
      draft = '';
      await generateAnswer(sessionId, message.id, epoch);
    } catch (cause) {
      if (epoch === chatEpoch) chatError = String(cause) + ' Tente o mesmo envio novamente; a tentativa mantém seu identificador.';
    } finally {
      if (epoch === chatEpoch) { partial = ''; chatBusy = false; }
    }
  }

  async function retryChat() {
    if (!retryAvailable || chatBusy || chatLoading || !selected || !hasKey || !ownerStatus?.unlocked) return;
    const epoch = chatEpoch;
    const sessionId = selected;
    const userId = retryMessageId;
    chatBusy = true;
    chatError = '';
    retryMessageId = '';
    try {
      const history = await invoke<ChatMessage[]>('chat_history', { sessionId });
      if (epoch !== chatEpoch) return;
      messages = history;
      if (history.at(-1)?.id !== userId || history.at(-1)?.role !== 'user') {
        chatError = 'O histórico mudou. Nenhuma mensagem foi reenviada.';
        return;
      }
      await generateAnswer(sessionId, userId, epoch);
    } catch (cause) {
      if (epoch === chatEpoch) chatError = String(cause);
    } finally {
      if (epoch === chatEpoch) { partial = ''; chatBusy = false; }
    }
  }

  async function cancelChat() {
    if (!generating || cancelling) return;
    const epoch = chatEpoch;
    cancelling = true;
    try { await invoke('chat_cancel', { runId }); }
    catch (cause) {
      if (epoch === chatEpoch) { chatError = String(cause); cancelling = false; }
    }
  }

  function clearPrivateState() {
    chatEpoch++;
    chats = []; messages = []; selected = ''; partial = ''; draft = ''; apiKey = '';
    hasKey = false; chatBusy = false; chatLoading = false; generating = false;
    cancelling = false; retryMessageId = ''; runId = ''; renaming = false; renameDraft = '';
    chatError = ''; info = null; ownerPassword = ''; ownerConfirm = ''; pendingSend = null;
    view = 'chat';
  }

  async function verifySession() {
    if (ownerBusy || sessionCheckRunning || !ownerStatus?.unlocked) return;
    sessionCheckRunning = true;
    const epoch = ownerEpoch;
    try {
      const status = await invoke<OwnerStatus>('owner_status');
      if (epoch !== ownerEpoch) return;
      if (!status.unlocked) {
        ownerBusy = true;
        ownerEpoch++;
        clearPrivateState();
        ownerStatus = status;
        ownerError = 'Sua sessão expirou. Entre novamente.';
        await invoke('lock_owner').catch(() => {});
        ownerBusy = false;
      }
    } catch {
      if (epoch !== ownerEpoch) return;
      ownerBusy = true;
      ownerEpoch++;
      clearPrivateState();
      ownerStatus = { configured: true, unlocked: false, displayName: null };
      ownerError = 'Não foi possível verificar a sessão. Entre novamente.';
      await invoke('lock_owner').catch(() => {});
      ownerBusy = false;
    } finally {
      sessionCheckRunning = false;
    }
  }

  async function refreshOwner() {
    const epoch = ownerEpoch;
    try {
      const status = await invoke<OwnerStatus>('owner_status');
      if (epoch !== ownerEpoch) return;
      if (!status.unlocked) clearPrivateState();
      ownerStatus = status;
      ownerError = null;
      if (status.unlocked) await loadChat();
    } catch (cause) {
      if (epoch !== ownerEpoch) return;
      clearPrivateState();
      ownerStatus = null;
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
    ownerEpoch++;
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
    if (ownerBusy) return;
    ownerBusy = true;
    ownerEpoch++;
    clearPrivateState();
    ownerStatus = { configured: true, unlocked: false, displayName: null };
    try {
      await invoke('lock_owner');
      await refreshOwner();
      await loadBootstrap();
    } catch (cause) {
      ownerError = String(cause);
    } finally {
      ownerBusy = false;
    }
  }

  async function loadBootstrap() {
    try {
      const epoch = ownerEpoch;
      const result = await invoke<BootstrapInfo>('bootstrap');
      if (epoch !== ownerEpoch) return;
      info = result;
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
    const interval = window.setInterval(() => void verifySession(), 1000);
    const onFocus = () => void verifySession();
    window.addEventListener('focus', onFocus);
    document.addEventListener('visibilitychange', onFocus);

    return () => {
      disposed = true;
      window.clearInterval(interval);
      window.removeEventListener('focus', onFocus);
      document.removeEventListener('visibilitychange', onFocus);
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
    <section class="workspace" aria-label="Área principal">
      <nav class="app-nav" aria-label="Navegação principal">
        <button type="button" class:active={view === 'chat'} aria-current={view === 'chat' ? 'page' : undefined} onclick={() => (view = 'chat')}>Chat</button>
        <button type="button" class:active={view === 'organizations'} aria-current={view === 'organizations' ? 'page' : undefined} onclick={() => (view = 'organizations')}>Organizações</button>
        <button type="button" class:active={view === 'tools'} aria-current={view === 'tools' ? 'page' : undefined} onclick={() => (view = 'tools')}>Tools</button>
        <button type="button" class:active={view === 'pessoal'} aria-current={view === 'pessoal' ? 'page' : undefined} onclick={() => (view = 'pessoal')}>Pessoal</button>
      </nav>

      {#if view === 'chat'}
      <section class="chat-shell" aria-label="Chat da Tina">
        <aside>
          <h2>Conversas</h2>
          <button disabled={chatBusy || chatLoading || !!pendingSend} onclick={() => { chatEpoch++; selected = ''; messages = []; partial = ''; draft = ''; retryMessageId = ''; chatError = ''; renaming = false; renameDraft = ''; }}>Nova conversa</button>
          {#each chats as chat (chat.id)}
            <button disabled={chatBusy || chatLoading || !!pendingSend} onclick={() => openChat(chat.id)} aria-pressed={selected === chat.id}>{chat.title}</button>
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
              <button disabled={chatBusy || chatLoading || !!pendingSend} onclick={() => { renameDraft = chats.find((chat) => chat.id === selected)?.title ?? ''; renaming = true; }}>Renomear conversa</button>
            {/if}
          {/if}
          {#if !hasKey}
            <form onsubmit={saveKey}>
              <label for="nvidia-key">Chave NVIDIA NIM</label>
              <input id="nvidia-key" type="password" autocomplete="off" required disabled={chatLoading || chatBusy} bind:value={apiKey} />
              <button type="submit" disabled={chatLoading || chatBusy || !apiKey.trim()}>Salvar no cofre Windows</button>
            </form>
          {/if}
          {#key selected}
            <ChatHistory {messages} {partial} loading={chatLoading} {generating} {cancelling} greeting={initialGreeting} />
          {/key}
          {#if selected}
            <DepartmentProposal sessionId={selected} lastMessageId={messages.at(-1)?.id ?? ''} ondecision={(message) => { messages = [...messages.filter(m => m.id !== message.id), message]; }} />
          {/if}
          {#if chatError}<p role="alert" class="error">{chatError}</p>{/if}
          {#if retryAvailable && !chatBusy}
            <button disabled={chatLoading || !hasKey} onclick={retryChat}>Tentar resposta novamente</button>
          {/if}
          <ChatComposer bind:draft disabled={chatLoading || !hasKey} readOnly={!!pendingSend} busy={chatBusy} {generating} {cancelling} onSend={sendChat} onCancel={cancelChat} />
        </div>
      </section>
      {:else if view === 'organizations'}
        <OrganizationDirectory />
      {:else if view === 'tools'}
        <ToolsDirectory />
      {:else}
        <section class="pessoal-shell" aria-label="Pessoal">
          <PersonnelDirectory />
        </section>
      {/if}
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
  .workspace { display: flex; flex-direction: column; gap: 1rem; }
  .app-nav { display: inline-flex; flex-wrap: wrap; gap: .5rem; align-self: flex-start; padding: .3rem; border: 1px solid var(--tina-border); border-radius: var(--tina-radius-pill); background: var(--tina-surface-tint); }
  .app-nav button { border: 0; background: transparent; color: var(--tina-text-muted); padding: .55rem .95rem; border-radius: var(--tina-radius-pill); font: inherit; font-weight: 600; cursor: pointer; }
  .app-nav button.active { background: #fff; color: var(--tina-pink-deep); box-shadow: 0 1px 0 rgba(0,0,0,.02); }
  .app-nav button:focus-visible { outline: 2px solid var(--tina-pink); outline-offset: 2px; }
  .chat-shell { display: grid; grid-template-columns: minmax(130px, 220px) 1fr; gap: 1rem; }
  .chat-shell aside, .chat-main { display: flex; flex-direction: column; gap: .75rem; min-width: 0; padding: 1rem; border: 1px solid var(--tina-border); border-radius: 12px; background: var(--tina-surface-tint); }
  .chat-main form { display: flex; flex-direction: column; gap: .5rem; }
  .chat-shell button, .chat-shell input { padding: .65rem; border: 1px solid var(--tina-border); border-radius: 8px; font: inherit; }
  .pessoal-shell { max-width: 52rem; }
  @media (max-width: 640px) { .chat-shell { grid-template-columns: 1fr; } .app-nav { width: 100%; } .app-nav button { flex: 1; } }

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
