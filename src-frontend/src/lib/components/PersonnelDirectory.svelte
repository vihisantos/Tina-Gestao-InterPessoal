<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import EmployeeSpace from './EmployeeSpace.svelte';
  import EmployeeProfile from './EmployeeProfile.svelte';
  import EmployeeRun from './EmployeeRun.svelte';
  import type { EmployeeDirectoryEntry } from '../personnel';

  let employees = $state<EmployeeDirectoryEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | undefined>();
  let selectedId = $state('');
  let selected = $derived(employees.find(employee => employee.id === selectedId));
  let generation = 0;

  async function refresh() {
    const current = ++generation;
    employees = [];
    selectedId = '';
    error = undefined;
    loading = true;
    try {
      const rows = await invoke<EmployeeDirectoryEntry[]>('list_employee_directory');
      if (current === generation) employees = rows;
    } catch {
      if (current === generation) error = 'Não foi possível carregar os funcionários. Tente atualizar ou entre novamente.';
    } finally {
      if (current === generation) loading = false;
    }
  }

  onMount(() => {
    void refresh();
    return () => { generation++; };
  });
</script>

<div class="directory-actions">
  <button type="button" onclick={refresh} disabled={loading}>Atualizar funcionários</button>
</div>
{#if selected}
  <button type="button" onclick={() => (selectedId = '')}>Voltar à lista</button>
  <EmployeeProfile name={selected.name} position={selected.position} department={selected.department}
    company={selected.company} active={selected.active} supervisor={selected.supervisor ?? undefined}
    responsibilities={selected.responsibilities ?? undefined} />
  {#if selected.active}{#key selected.id}<EmployeeRun employeeId={selected.id} />{/key}{/if}
{:else}
  <EmployeeSpace {employees} {loading} {error} onopen={id => (selectedId = id)} />
{/if}

<style>
  .directory-actions { display: flex; justify-content: flex-end; margin-bottom: 1rem; }
  button { font: inherit; padding: .65rem; border: 1px solid var(--tina-border); border-radius: var(--tina-radius-lg); background: var(--tina-surface); color: inherit; }
  button:focus-visible { outline: 2px solid var(--tina-pink); }
</style>
