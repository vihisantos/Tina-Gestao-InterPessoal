<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import Card from './Card.svelte';
  type Message = { id: string; role: string; content: string; created_at: string };
  type Proposal = { id: string; tool_name: string; reason?: string | null; company_name: string; organization_kind: string; name: string; code: string; description?: string | null };
  let { sessionId, lastMessageId, ondecision }: { sessionId: string; lastMessageId: string; ondecision: (message: Message) => void } = $props();
  let proposal = $state<Proposal | null>(null);
  let busy = $state(false);
  let error = $state('');
  let epoch = 0;
  $effect(() => {
    const session = sessionId;
    const last = lastMessageId;
    const current = ++epoch;
    proposal = null;
    error = '';
    if (session && last) {
      invoke<Proposal | null>('chat_pending_department', { sessionId: session })
        .then(result => { if (current === epoch) proposal = result; })
        .catch(() => { if (current === epoch) error = 'Não foi possível consultar a proposta.'; });
    }
    return () => { epoch++; };
  });
  async function decide(approved: boolean) {
    if (!proposal || busy) return;
    const current = epoch;
    busy = true;
    error = '';
    try {
      const result = await invoke<Message>('chat_decide_department', { sessionId, proposalId: proposal.id, approved });
      if (current === epoch) { proposal = null; ondecision(result); }
    } catch (cause) { if (current === epoch) error = String(cause); }
    finally { busy = false; }
  }
</script>
{#if proposal}
  <Card title={proposal.tool_name.startsWith('assign_') ? 'Autorizar atribuição de ferramentas?' : proposal.tool_name.startsWith('revoke_') ? 'Autorizar revogação de ferramentas?' : proposal.tool_name === 'create_organization' ? 'Posso criar esta organização?' : proposal.tool_name === 'create_position' ? 'Posso criar este cargo?' : proposal.tool_name === 'generate_contract' ? 'Posso salvar esta minuta de contrato?' : 'Posso criar este departamento?'}>
    <p><strong>{proposal.name}</strong>{#if proposal.code} ({proposal.code}){/if}</p>
    <p>{proposal.tool_name !== 'create_organization' ? `Organização: ${proposal.company_name} — ` : 'Contexto: '}{proposal.organization_kind === 'personal' ? 'Vida pessoal' : 'Empresarial'}</p>
    {#if proposal.reason}<p><strong>Por que a Tina sugere:</strong> {proposal.reason}</p>{/if}
    {#if proposal.description}<p class="description">{proposal.description}</p>{/if}
    <p>A alteração será registrada no banco somente após sua autorização.</p>
    <div class="actions">
      <button type="button" disabled={busy} onclick={() => decide(true)}>Confirmar</button>
      <button type="button" disabled={busy} onclick={() => decide(false)}>Recusar</button>
    </div>
  </Card>
{/if}
{#if error}<p role="alert">{error}</p>{/if}
<style>
  .description { white-space: pre-wrap; overflow-wrap: anywhere; }
  .actions { display: flex; flex-wrap: wrap; gap: .5rem; }
  button { font: inherit; padding: .65rem; border: 1px solid var(--tina-border-strong); border-radius: var(--tina-radius-sm); background: var(--tina-surface); color: inherit; }
  button:focus-visible { outline: 2px solid var(--tina-pink); }
</style>
