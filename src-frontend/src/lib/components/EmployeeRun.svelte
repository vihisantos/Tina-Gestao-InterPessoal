<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  let { employeeId }: { employeeId: string } = $props();
  type Result = { answer: string; model: string; tool: string | null; inference_ms: number };
  let task = $state('');
  let result = $state<Result | null>(null);
  let busy = $state(false);
  let error = $state('');
  let generation = 0;
  $effect(() => { const id = employeeId; if (id) { generation++; task = ''; result = null; error = ''; busy = false; } return () => { generation++; }; });
  async function run() {
    if (busy || !task.trim()) return;
    const current = generation;
    const id = employeeId;
    busy = true; error = ''; result = null;
    try {
      const response = await invoke<Result>('run_employee', { employeeId: id, task });
      if (current === generation) result = response;
    } catch (cause) { if (current === generation) error = String(cause); }
    finally { if (current === generation) busy = false; }
  }
</script>
<section aria-label="Execução experimental">
  <h3>Executar tarefa experimental</h3>
  <p>Utiliza o modelo cadastrado via NVIDIA; ainda não representa contratação validada. A única ferramenta disponível é consulta de cargos, se concedida. A tarefa será enviada à NVIDIA.</p>
  <label>Instrução<textarea bind:value={task} maxlength={2000} disabled={busy} rows={4}></textarea></label>
  <button type="button" onclick={run} disabled={busy || !task.trim()}>{busy ? 'Executando…' : 'Executar tarefa'}</button>
  {#if error}<p role="alert">{error}</p>{/if}
  {#if result}
    <p>Modelo: {result.model} — latência de inferência: {result.inference_ms} ms.</p>
    <p>Ferramenta executada: {result.tool ?? 'Nenhuma'}</p>
    <p class="answer">{result.answer}</p>
  {/if}
</section>
<style>
  label { display: grid; gap: .5rem; }
  textarea, button { font: inherit; color: inherit; background: var(--tina-surface); border: 1px solid var(--tina-border); padding: .65rem; border-radius: var(--tina-radius-sm); }
  .answer { white-space: pre-wrap; overflow-wrap: anywhere; }
</style>
