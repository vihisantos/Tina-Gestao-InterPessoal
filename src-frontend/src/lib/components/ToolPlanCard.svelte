<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import Badge from './Badge.svelte';
  import Button from './Button.svelte';
  import Card from './Card.svelte';

  type Message = { id: string; role: string; content: string; created_at: string };
  type PlanStep = {
    stepIndex: number;
    toolCallId: string;
    toolName: string;
    effects: string[];
    dependsOn: string[];
    state: string;
  };
  type PlanCard = {
    id: string;
    state: string;
    fingerprint: string;
    steps: PlanStep[];
    limits: { maxCalls: number; maxRounds: number; maxTokensEstimate: number };
    rounds: number;
  };
  type PlanDecision = {
    planId: string;
    state: string;
    card: PlanCard | null;
    executed: number;
    failed: string[];
    skipped: string[];
    message: Message;
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

  let plan = $state<PlanCard | null>(null);
  let busy = $state(false);
  let error = $state('');
  let epoch = 0;
  let currentSession = '';

  const awaiting = $derived(plan?.state === 'draft' || plan?.state === 'awaiting_authorization');
  const resumable = $derived(plan?.state === 'authorized' || plan?.state === 'running');

  function describe(cause: unknown): string {
    return cause instanceof Error ? cause.message : String(cause);
  }

  function refresh(session: string) {
    const current = ++epoch;
    if (!session) {
      plan = null;
      return;
    }
    invoke<PlanCard | null>('chat_pending_plan', { sessionId: session })
      .then(result => {
        if (current === epoch && session === sessionId) {
          plan = result;
          error = '';
        }
      })
      .catch(cause => {
        if (current === epoch) error = describe(cause) || 'Não foi possível consultar o plano.';
      });
  }

  // Troca de sessão limpa o cartão; mensagens novas recarregam o plano ativo.
  $effect(() => {
    const session = sessionId;
    const trigger = lastMessageId;
    if (session !== currentSession) {
      currentSession = session;
      plan = null;
      error = '';
    }
    if (session && trigger) refresh(session);
  });

  async function decide(approved: boolean) {
    if (!plan || busy) return;
    const current = epoch;
    const id = plan.id;
    const fingerprint = plan.fingerprint;
    busy = true;
    error = '';
    try {
      const decision = await invoke<PlanDecision>('chat_decide_plan', {
        sessionId,
        planId: id,
        fingerprint,
        approved
      });
      if (current === epoch) {
        plan = null;
        ondecision(decision.message);
      }
    } catch (cause) {
      if (current === epoch) error = describe(cause);
    } finally {
      busy = false;
    }
  }

  async function resume() {
    if (!plan || busy || !resumable) return;
    const current = epoch;
    busy = true;
    error = '';
    try {
      const decision = await invoke<PlanDecision>('chat_resume_plan', {
        sessionId,
        planId: plan.id
      });
      if (current === epoch) {
        plan = null;
        ondecision(decision.message);
      }
    } catch (cause) {
      if (current === epoch) error = describe(cause);
    } finally {
      busy = false;
    }
  }

  async function cancel() {
    if (!plan || busy) return;
    const current = epoch;
    busy = true;
    error = '';
    try {
      const decision = await invoke<PlanDecision>('chat_cancel_plan', {
        sessionId,
        planId: plan.id,
        reason: 'Cancelado explicitamente pelo proprietário'
      });
      if (current === epoch) {
        plan = null;
        ondecision(decision.message);
      }
    } catch (cause) {
      if (current === epoch) error = describe(cause);
    } finally {
      busy = false;
    }
  }

  async function supersede() {
    if (!plan || busy) return;
    const current = epoch;
    busy = true;
    error = '';
    try {
      const decision = await invoke<PlanDecision>('chat_supersede_plan', {
        sessionId,
        planId: plan.id,
        reason: 'Substituído explicitamente pelo proprietário'
      });
      if (current === epoch) {
        plan = null;
        ondecision(decision.message);
      }
    } catch (cause) {
      if (current === epoch) error = describe(cause);
    } finally {
      busy = false;
    }
  }

  function label(toolName: string): string {
    const names: Record<string, string> = {
      create_organization: 'Criar organização',
      create_department: 'Criar departamento',
      create_position: 'Criar cargo',
      generate_contract: 'Preparar minuta',
      assign_tool: 'Conceder ferramenta',
      assign_app: 'Conceder aplicativo',
      revoke_tool: 'Revogar ferramenta',
      revoke_app: 'Revogar aplicativo'
    };
    return names[toolName] ?? toolName;
  }
</script>

{#if plan}
  <Card title="Plano de ações da Tina">
    {#snippet badge()}
      <Badge tone="ok">
        {awaiting ? 'Aguardando autorização' : resumable ? 'Autorizado' : (plan?.state ?? 'indefinido')}
      </Badge>
    {/snippet}
    <p class="summary">
      {plan.steps.length} ação(ões) em sequência · até {plan.limits.maxCalls} chamadas ·
      até {plan.limits.maxRounds} rodadas · estimativa de {plan.limits.maxTokensEstimate} tokens.
    </p>
    <p class="detail">Versão autorizável: <code>{plan.fingerprint.slice(0, 16)}</code></p>
    <ol class="steps">
      {#each plan.steps as step (step.toolCallId)}
        <li>
          <p><strong>{step.stepIndex + 1}. {label(step.toolName)}</strong> <span class="muted">{step.state}</span></p>
          {#each step.effects as effect}
            <p class="effect">{effect}</p>
          {/each}
          {#if step.dependsOn.length}
            <p class="detail">Depende de: {step.dependsOn.join(', ')}</p>
          {/if}
        </li>
      {/each}
    </ol>
    <p class="notice">
      Nenhuma ação será executada antes da sua autorização. Etapas já concluídas nunca são repetidas;
      falhas parciais não desfazem efeitos gravados.
    </p>
    <div class="actions">
      {#if awaiting}
        <Button onclick={() => decide(true)} disabled={busy}>Aprovar e executar</Button>
        <Button variant="secondary" onclick={() => decide(false)} disabled={busy}>Recusar</Button>
      {:else if resumable}
        <Button onclick={resume} disabled={busy}>Continuar plano</Button>
      {/if}
      <Button variant="secondary" onclick={cancel} disabled={busy}>Cancelar</Button>
      <Button variant="secondary" onclick={supersede} disabled={busy}>Substituir explicitamente</Button>
    </div>
  </Card>
{/if}
{#if error}<p role="alert" class="error">{error}</p>{/if}

<style>
  .summary { margin: 0 0 0.4rem; }
  .detail { margin: 0.25rem 0; color: var(--tina-text-muted); font-size: 0.82rem; }
  .muted { color: var(--tina-text-muted); font-size: 0.78rem; }
  .steps { margin: 0.6rem 0; padding-left: 1.25rem; display: grid; gap: 0.55rem; }
  .steps li { padding: 0.45rem 0.6rem; border: 1px solid var(--tina-border); border-radius: var(--tina-radius-pill); background: var(--tina-surface-tint); }
  .steps p { margin: 0; }
  .effect { margin-top: 0.2rem; font-size: 0.84rem; overflow-wrap: anywhere; }
  .notice { color: var(--tina-text-muted); font-size: 0.82rem; }
  .actions { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .error { color: var(--tina-pink-deep); font-size: 0.85rem; }
</style>
