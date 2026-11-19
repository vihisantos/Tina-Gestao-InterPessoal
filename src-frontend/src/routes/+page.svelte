<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
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
</main>

<style>
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
