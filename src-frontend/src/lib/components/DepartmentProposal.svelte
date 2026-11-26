<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import Badge from './Badge.svelte';
  import Button from './Button.svelte';
  import Card from './Card.svelte';

  type Message = { id: string; role: string; content: string; created_at: string };
  type Proposal = {
    id: string;
    tool_name: string;
    lifecycle_status?: string;
    reason?: string | null;
    company_name: string;
    organization_kind: string;
    name: string;
    code: string;
    description?: string | null;
  };

  let {
    sessionId,
    lastMessageId,
    ondecision
  }: {
    sessionId: string;
    lastMessageId: string;
    ondecision: (message: Message) => void;
  } = $props();

  let proposal = $state<Proposal | null>(null);
  let busy = $state(false);
  let error = $state('');
  let epoch = 0;
  let currentSession = '';

  function describe(cause: unknown): string {
    return cause instanceof Error ? cause.message : String(cause);
  }

  function refresh(session: string) {
    const current = ++epoch;
    if (!session) {
      proposal = null;
      return;
    }
    invoke<Proposal | null>('chat_pending_department', { sessionId: session })
      .then(result => {
        if (current === epoch && session === sessionId) proposal = result;
      })
      .catch(cause => {
        if (current === epoch) error = describe(cause) || 'Não foi possível consultar a proposta.';
      });
  }

  // Troca de sessão limpa o cartão; nova mensagem só recarrega a proposta ativa.
  $effect(() => {
    const session = sessionId;
    const trigger = lastMessageId;
    if (session !== currentSession) {
      currentSession = session;
      proposal = null;
      error = '';
    }
    if (session && trigger) refresh(session);
  });

  async function decide(approved: boolean) {
    if (!proposal || busy) return;
    const current = epoch;
    const id = proposal.id;
    busy = true;
    error = '';
    try {
      const message = await invoke<Message>('chat_decide_department', {
        sessionId,
        proposalId: id,
        approved
      });
      if (current === epoch) {
        proposal = null;
        ondecision(message);
      }
    } catch (cause) {
      if (current === epoch) error = describe(cause);
    } finally {
      busy = false;
    }
  }

  async function cancel() {
    if (!proposal || busy) return;
    const current = epoch;
    const id = proposal.id;
    busy = true;
    error = '';
    try {
      const message = await invoke<Message>('chat_cancel_proposal', {
        sessionId,
        proposalId: id
      });
      if (current === epoch) {
        proposal = null;
        ondecision(message);
      }
    } catch (cause) {
      if (current === epoch) error = describe(cause);
    } finally {
      busy = false;
    }
  }
</script>

{#if proposal}
  <Card title={proposal.tool_name === 'create_organization' ? 'Posso criar esta organização?' : 'Posso criar este departamento?'}>
    {#snippet badge()}<Badge tone="ok">Aguardando decisão</Badge>{/snippet}
    <p><strong>{proposal.name}</strong>{#if proposal.code} ({proposal.code}){/if}</p>
    <p>{proposal.tool_name === 'create_department' ? `Organização: ${proposal.company_name} — ` : 'Contexto: '}{proposal.organization_kind === 'personal' ? 'Vida pessoal' : 'Empresarial'}</p>
    {#if proposal.reason}<p><strong>Por que a Tina sugere:</strong> {proposal.reason}</p>{/if}
    {#if proposal.description}<p>{proposal.description}</p>{/if}
    <p>A criação será registrada no banco somente após sua autorização. Você também pode cancelar sem criar nada.</p>
    <div class="actions">
      <Button onclick={() => decide(true)} disabled={busy}>Confirmar criação</Button>
      <Button variant="secondary" onclick={() => decide(false)} disabled={busy}>Recusar</Button>
      <Button variant="secondary" onclick={cancel} disabled={busy}>Cancelar proposta</Button>
    </div>
  </Card>
{/if}
{#if error}<p role="alert" class="error">{error}</p>{/if}

<style>
  .actions { display: flex; flex-wrap: wrap; gap: .5rem; }
  .error { color: var(--tina-pink-deep); font-size: .85rem; }
</style>
