<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

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
</svelte:head>

<main>
  <header>
    <div class="brand">
      <span class="mark">T</span>
      <div>
        <p class="eyebrow">TINA · GESTÃO INTERPESSOAL</p>
        <h1>A sua operação, sob sua autoridade.</h1>
      </div>
    </div>
    <span class="pill">{info ? `v${info.version}` : 'conectando…'}</span>
  </header>

  <section class="intro">
    <p>
      Primeira versão executável da Tina. Ela abre em uma janela desktop real,
      mantém todos os dados na máquina e já conversa com o backend Rust por
      comandos e eventos nativos do Tauri.
    </p>
    {#if error}
      <p class="error" role="alert">Falha de comunicação: {error}</p>
    {/if}
  </section>

  <section class="grid" aria-label="Estado da instalação">
    <article class="card">
      <div class="card-top">
        <h2>Ambiente desktop</h2>
        <span class="badge ok">Tauri 2</span>
      </div>
      <p class="value">{info ? info.message : 'Lendo estado local…'}</p>
      <p class="detail">
        Evento <code>tina://ready</code>
        <strong class:ok-text={readyEvent}>{readyEvent ? 'recebido' : 'aguardando'}</strong>
      </p>
    </article>

    <article class="card">
      <div class="card-top">
        <h2>Banco local</h2>
        <span class="badge" class:ok={info?.databaseReady}>{info?.databaseReady ? 'pronto' : '…'}</span>
      </div>
      <p class="value path">{info?.databasePath ?? 'Localizando %APPDATA%\\tina\\tina.db…'}</p>
      <p class="detail">SQLite criado e migrado na inicialização, sem serviço externo.</p>
    </article>

    <article class="card">
      <div class="card-top">
        <h2>Ponte Rust ↔ Svelte</h2>
        <span class="badge" class:ok={pingResult === 'pong'}>{pingResult === 'pong' ? 'ativa' : 'testar'}</span>
      </div>
      <p class="value">{pingResult ? `invoke('ping') → ${pingResult}` : 'Nenhum teste enviado ainda.'}</p>
      <button onclick={checkBridge}>Verificar comunicação</button>
    </article>
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
  :global(body) {
    margin: 0;
    background: #0d1117;
    color: #e6edf3;
    font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
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

  .mark {
    display: grid;
    place-items: center;
    width: 3rem;
    height: 3rem;
    border-radius: 0.9rem;
    background: linear-gradient(135deg, #4f8cff, #7b61ff);
    font-weight: 700;
    font-size: 1.5rem;
  }

  .eyebrow {
    margin: 0 0 0.25rem;
    color: #8b949e;
    font-size: 0.72rem;
    letter-spacing: 0.12em;
  }

  h1 {
    margin: 0;
    font-size: 1.8rem;
    line-height: 1.2;
  }

  .pill {
    padding: 0.4rem 0.8rem;
    border: 1px solid #30363d;
    border-radius: 999px;
    color: #8b949e;
    font-size: 0.8rem;
    white-space: nowrap;
  }

  .intro {
    border-left: 3px solid #4f8cff;
    padding-left: 1rem;
    margin-bottom: 2rem;
    color: #c9d1d9;
  }

  .intro p {
    margin: 0;
    line-height: 1.6;
  }

  .error {
    margin-top: 0.75rem !important;
    color: #ff7b72;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 1rem;
  }

  .card {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 0.9rem;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-height: 190px;
  }

  .card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  h2 {
    margin: 0;
    font-size: 1rem;
    color: #8b949e;
    font-weight: 600;
  }

  .badge {
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    background: #21262d;
    color: #8b949e;
    font-size: 0.72rem;
    text-transform: uppercase;
  }

  .badge.ok {
    background: rgba(46, 160, 67, 0.18);
    color: #56d364;
  }

  .value {
    margin: 0;
    font-size: 1.05rem;
    line-height: 1.4;
  }

  .path {
    font-family: Consolas, 'Cascadia Code', monospace;
    font-size: 0.82rem;
    word-break: break-all;
    color: #79c0ff;
  }

  .detail {
    margin: 0;
    color: #8b949e;
    font-size: 0.85rem;
    line-height: 1.5;
  }

  .detail strong {
    color: #d29922;
  }

  .detail strong.ok-text {
    color: #56d364;
  }

  code {
    background: #21262d;
    padding: 0.1rem 0.3rem;
    border-radius: 0.25rem;
  }

  button {
    margin-top: auto;
    align-self: flex-start;
    padding: 0.55rem 0.9rem;
    border: 1px solid #4f8cff;
    border-radius: 0.5rem;
    background: rgba(79, 140, 255, 0.12);
    color: #c9e1ff;
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
  }

  button:hover {
    background: rgba(79, 140, 255, 0.25);
  }

  footer {
    margin-top: 2rem;
    padding-top: 1rem;
    border-top: 1px solid #21262d;
    color: #6e7681;
    font-size: 0.82rem;
  }
</style>
