<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import Card from './Card.svelte';
  type Organization = { id: string; name: string; kind: 'personal' | 'business' };
  type Department = { id: string; name: string; code: string };
  let rows = $state<{ organization: Organization; departments: Department[] }[]>([]);
  let loading = $state(false);
  let error = $state('');
  let name = $state('');
  let kind = $state<'personal' | 'business'>('business');
  let busy = $state(false);
  let epoch = 0;
  async function refresh() {
    const current = ++epoch;
    loading = true;
    error = '';
    try {
      const organizations = await invoke<Organization[]>('list_companies');
      const result = await Promise.all(organizations.map(async organization => ({
        organization,
        departments: await invoke<Department[]>('list_departments', { companyId: organization.id })
      })));
      if (current === epoch) rows = result;
    } catch (cause) { if (current === epoch) error = String(cause); }
    finally { if (current === epoch) loading = false; }
  }
  async function create(event: SubmitEvent) {
    event.preventDefault();
    if (busy || !name.trim()) return;
    busy = true;
    error = '';
    try {
      await invoke('create_company', { request: { name: name.trim(), kind } });
      name = '';
      await refresh();
    } catch (cause) { error = String(cause); }
    finally { busy = false; }
  }
  onMount(() => { void refresh(); return () => { epoch++; }; });
</script>
<Card title="Organizações">
  <form onsubmit={create}>
    <label for="org-kind">Contexto</label>
    <select id="org-kind" bind:value={kind}>
      <option value="personal">Vida pessoal</option>
      <option value="business">Empresarial</option>
    </select>
    <label for="org-name">Nome</label>
    <input id="org-name" bind:value={name} placeholder={kind === 'personal' ? 'Pessoal' : 'Nome da empresa'} maxlength="160" required />
    <button type="submit" disabled={busy || !name.trim()}>Cadastrar organização</button>
  </form>
  <button type="button" onclick={refresh} disabled={loading}>Atualizar</button>
  {#if error}<p role="alert">{error}</p>{/if}
  {#if loading}<p>Carregando organizações…</p>
  {:else if rows.length === 0}<p>Nenhuma organização cadastrada.</p>
  {:else}
    {#each rows as row (row.organization.id)}
      <h3>{row.organization.name} — {row.organization.kind === 'personal' ? 'Vida pessoal' : 'Empresarial'}</h3>
      {#if row.departments.length === 0}<p>Nenhum departamento.</p>
      {:else}<ul>{#each row.departments as department (department.id)}<li>{department.name} ({department.code})</li>{/each}</ul>{/if}
    {/each}
  {/if}
</Card>
<style>
  form { display: grid; gap: .5rem; margin-bottom: 1rem; }
  input, select, button { font: inherit; padding: .65rem; border: 1px solid var(--tina-border); border-radius: var(--tina-radius-sm); background: var(--tina-surface); color: inherit; }
  button:focus-visible { outline: 2px solid var(--tina-pink); }
</style>
