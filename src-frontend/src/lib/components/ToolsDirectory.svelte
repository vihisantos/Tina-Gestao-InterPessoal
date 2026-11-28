<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import Card from './Card.svelte';
  import type { EmployeeDirectoryEntry } from '../personnel';
  type Tool = { key: string; app: string; label: string; assignable: boolean };
  type Grant = { employee_id: string; company_id: string; department_id: string; tool_key: string };
  const apps = [{ key: 'rh', name: 'RH' }, { key: 'organization', name: 'Estrutura organizacional' }, { key: 'administration', name: 'Administração de acessos' }];
  let tools = $state<Tool[]>([]);
  let employees = $state<EmployeeDirectoryEntry[]>([]);
  let grants = $state<Grant[]>([]);
  let employeeId = $state('');
  let selectedApp = $state('rh');
  let busy = $state(false);
  let loading = $state(true);
  let error = $state('');
  let epoch = 0;
  let appTools = $derived(tools.filter(tool => tool.app === selectedApp));
  let chosen = $derived(employees.find(e => e.id === employeeId));
  function assigned(key: string) {
    return grants.some(g => g.employee_id === employeeId && g.tool_key === key);
  }
  async function refresh() {
    const current = ++epoch;
    loading = true; error = '';
    try {
      const [t, e, g] = await Promise.all([
        invoke<Tool[]>('list_tool_catalog'),
        invoke<EmployeeDirectoryEntry[]>('list_employee_directory'),
        invoke<Grant[]>('list_tool_grants')
      ]);
      if (current !== epoch) return;
      tools = t; employees = e; grants = g;
      if (!employees.some(row => row.id === employeeId)) employeeId = '';
    } catch (cause) { if (current === epoch) error = String(cause); }
    finally { if (current === epoch) loading = false; }
  }
  async function change(action: string, key: string) {
    if (!chosen || busy || loading) return;
    const id = chosen.id;
    const label = action.startsWith('assign') ? 'Atribuir' : 'Revogar';
    if (!window.confirm(label + ' ' + key + ' para ' + chosen.name + ' (' + chosen.company + ' / ' + chosen.department + ')?')) return;
    busy = true; error = '';
    try {
      await invoke('change_tool_access', { employeeId: id, action, key });
      await refresh();
    } catch (cause) { error = String(cause); }
    finally { busy = false; }
  }
  onMount(() => { void refresh(); return () => { epoch++; }; });
</script>
<Card title="Tools — aplicativos">
  <p>Aplicativos agrupam ferramentas. Atribua ou revogue cada ferramenta ou pacote. Acesso configurado não significa execução automática. No executor experimental, somente a consulta de cargos está habilitada.</p>
  <div class="apps" aria-label="Aplicativos">
    {#each apps as app}
      <button type="button" aria-pressed={selectedApp === app.key} onclick={() => selectedApp = app.key} disabled={busy}>{app.name}</button>
    {/each}
  </div>
  <label for="tool-employee">Funcionário</label>
  <select id="tool-employee" bind:value={employeeId} disabled={busy || loading}>
    <option value="">Selecione um funcionário</option>
    {#each employees as employee (employee.id)}
      <option value={employee.id}>{employee.name} — {employee.company} / {employee.department}{employee.active ? '' : ' (inativo)'}</option>
    {/each}
  </select>
  {#if error}<p role="alert">{error}</p>{/if}
  {#if loading}<p>Carregando catálogo…</p>
  {:else}
    {#if appTools.some(t => t.assignable)}
      <div class="actions">
        <button type="button" disabled={!chosen?.active || busy} onclick={() => change('assign_app', selectedApp)}>Atribuir aplicativo</button>
        <button type="button" disabled={!chosen || busy} onclick={() => change('revoke_app', selectedApp)}>Revogar aplicativo</button>
      </div>
    {/if}
    {#each appTools as tool (tool.key)}
      <div class="tool">
        <strong>{tool.label}</strong><small>{tool.key}</small>
        {#if tool.assignable}
          <span>{assigned(tool.key) ? 'Atribuída' : 'Não atribuída'}</span>
          <button type="button" disabled={!chosen || busy || (!assigned(tool.key) && !chosen.active)} onclick={() => change(assigned(tool.key) ? 'revoke_tool' : 'assign_tool', tool.key)}>{assigned(tool.key) ? 'Revogar' : 'Atribuir'}</button>
        {:else}<span>Exclusiva da administração central</span>{/if}
      </div>
    {/each}
    {#if selectedApp === 'rh'}<p>Minuta de contrato disponível na Tina Central. Avaliação sintética NVIDIA no backend; contratação ainda não disponível. A consulta de cargos pode ser executada experimentalmente por funcionário.</p>{/if}
  {/if}
  <button type="button" disabled={busy || loading} onclick={refresh}>Atualizar</button>
</Card>
<style>
  .apps,.actions { display: flex; flex-wrap: wrap; gap: .5rem; margin: .75rem 0; }
  .tool { display: grid; gap: .35rem; padding: .75rem 0; border-bottom: 1px solid var(--tina-border); }
  button,select { font: inherit; padding: .65rem; border: 1px solid var(--tina-border-strong); border-radius: var(--tina-radius-sm); background: var(--tina-surface); color: inherit; }
  button[aria-pressed="true"] { background: var(--tina-surface-tint); color: var(--tina-pink-deep); }
  button:focus-visible,select:focus-visible { outline: 2px solid var(--tina-pink); }
</style>
