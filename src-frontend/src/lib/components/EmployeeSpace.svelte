<script lang="ts">
  import EmployeeProfile from './EmployeeProfile.svelte';
  type Employee = { id: string; name: string; position: string; department: string; company: string; active: boolean; supervisor?: string; responsibilities?: string };
  let { employees, loading = false, error, onopen }: { employees: Employee[]; loading?: boolean; error?: string; onopen?: (id: string) => void } = $props();
  let query = $state('');
  let company = $state('');
  let status = $state('all');
  let companies = $derived([...new Set(employees.map(e => e.company))].sort((a,b) => a.localeCompare(b,'pt-BR')));
  const normalize = (value: string) => value.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLocaleLowerCase('pt-BR');
  let filtered = $derived(employees.filter(e => (!company || e.company === company) && (status === 'all' || e.active === (status === 'active')) && normalize([e.name,e.position,e.department,e.company].join(' ')).includes(normalize(query.trim()))));
  let groups = $derived([...new Set(filtered.map(e => e.company))].sort((a,b) => a.localeCompare(b,'pt-BR')).map(name => ({name, departments: [...new Set(filtered.filter(e => e.company === name).map(e => e.department))].sort((a,b) => a.localeCompare(b,'pt-BR')).map(department => ({name: department, employees: filtered.filter(e => e.company === name && e.department === department)}))})));
</script>

<section aria-label="Espaço dos funcionários">
  <header><h1>Funcionários</h1><p>Conheça e acompanhe os funcionários das suas organizações.</p></header>
  <div class="filters">
    <label>Buscar<input type="search" bind:value={query} placeholder="Nome, cargo ou departamento" /></label>
    <label>Empresa<select bind:value={company}><option value="">Todas</option>{#each companies as name (name)}<option value={name}>{name}</option>{/each}</select></label>
    <label>Status<select bind:value={status}><option value="all">Todos</option><option value="active">Ativos</option><option value="inactive">Inativos</option></select></label>
  </div>
  {#if loading}<p role="status">Carregando funcionários…</p>
  {:else if error}<p role="alert">{error}</p>
  {:else if employees.length === 0}<p>Nenhum funcionário cadastrado.</p>
  {:else if filtered.length === 0}<p>Nenhum funcionário corresponde aos filtros.</p>
  {:else}
    <p role="status">{filtered.length} de {employees.length} funcionários</p>
    {#each groups as group (group.name)}
      <section><h2>{group.name}</h2>
        {#each group.departments as department (department.name)}
          <h3>{department.name}</h3>
          <div class="profiles">
            {#each department.employees as employee (employee.id)}
              <EmployeeProfile name={employee.name} position={employee.position} department={employee.department} company={employee.company} active={employee.active} supervisor={employee.supervisor} responsibilities={employee.responsibilities} onopen={onopen ? () => onopen?.(employee.id) : undefined} />
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
