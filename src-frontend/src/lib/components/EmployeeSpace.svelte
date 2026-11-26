<script lang="ts">
  import EmployeeProfile from './EmployeeProfile.svelte';
  import type { EmployeeDirectoryEntry as Employee } from '../personnel';
  let { employees, loading = false, error, onopen }: { employees: Employee[]; loading?: boolean; error?: string; onopen?: (id: string) => void } = $props();
  let query = $state('');
  let company = $state('');
  let status = $state('all');
  function organizations(rows: Employee[], id: 'companyId' | 'departmentId', name: 'company' | 'department') {
    return [...new Map(rows.map(e => [e[id], { id: e[id], name: e[name] }])).values()]
      .sort((a, b) => a.name.localeCompare(b.name, 'pt-BR') || a.id.localeCompare(b.id));
  }
  let companies = $derived(organizations(employees, 'companyId', 'company'));
  const normalize = (value: string) => value.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLocaleLowerCase('pt-BR');
  let filtered = $derived(employees.filter(e => (!company || e.companyId === company) && (status === 'all' || e.active === (status === 'active')) && normalize([e.name,e.position,e.department,e.company].join(' ')).includes(normalize(query.trim()))));
  let groups = $derived(organizations(filtered, 'companyId', 'company').map(group => {
    const members = filtered.filter(e => e.companyId === group.id);
    return { ...group, departments: organizations(members, 'departmentId', 'department').map(department => ({
      ...department, employees: members.filter(e => e.departmentId === department.id)
    })) };
  }));
</script>

<section aria-label="Espaço dos funcionários">
  <header><h1>Funcionários</h1><p>Conheça e acompanhe os funcionários das suas organizações.</p></header>
  <div class="filters">
    <label>Buscar<input type="search" bind:value={query} placeholder="Nome, cargo ou departamento" /></label>
    <label>Empresa<select bind:value={company}><option value="">Todas</option>{#each companies as item (item.id)}<option value={item.id}>{item.name}</option>{/each}</select></label>
    <label>Status<select bind:value={status}><option value="all">Todos</option><option value="active">Ativos</option><option value="inactive">Inativos</option></select></label>
  </div>
  {#if loading}<p role="status">Carregando funcionários…</p>
  {:else if error}<p role="alert">{error}</p>
  {:else if employees.length === 0}<p>Nenhum funcionário cadastrado.</p>
  {:else if filtered.length === 0}<p>Nenhum funcionário corresponde aos filtros.</p>
  {:else}
    <p role="status">{filtered.length} de {employees.length} funcionários</p>
    {#each groups as group (group.id)}
      <section><h2>{group.name}</h2>
        {#each group.departments as department (department.id)}
          <h3>{department.name}</h3>
          <div class="profiles">
            {#each department.employees as employee (employee.id)}
              <EmployeeProfile name={employee.name} position={employee.position} department={employee.department} company={employee.company} active={employee.active} supervisor={employee.supervisor ?? undefined} responsibilities={employee.responsibilities ?? undefined} onopen={onopen ? () => onopen?.(employee.id) : undefined} />
            {/each}
          </div>
        {/each}
      </section>
    {/each}
  {/if}
</section>

<style>
  .filters, .profiles { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 16rem), 1fr)); gap: 1rem; }
  label { display: grid; gap: .35rem; font-size: .85rem; }
  input, select { font: inherit; padding: .65rem; border: 1px solid var(--tina-border); border-radius: var(--tina-radius-lg); background: var(--tina-surface); color: inherit; min-width: 0; }
  input:focus-visible, select:focus-visible { outline: 2px solid var(--tina-pink); }
  h1, h2, h3 { overflow-wrap: anywhere; }
</style>
