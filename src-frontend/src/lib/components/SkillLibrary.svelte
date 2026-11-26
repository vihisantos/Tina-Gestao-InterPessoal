<script lang="ts">
  import Card from './Card.svelte';
  import Badge from './Badge.svelte';
  import Button from './Button.svelte';
  type Skill = { id: string; name: string; objective: string; scope: string; version: number; status: 'draft' | 'validated' | 'published' | 'retired' };
  let { skills, loading = false, error, onopen }: { skills: Skill[]; loading?: boolean; error?: string; onopen?: (id: string) => void } = $props();
  const labels = { draft: 'Rascunho', validated: 'Validada', published: 'Publicada', retired: 'Descontinuada' };
  let query = $state('');
  let status = $state('all');
  let filtered = $derived(skills.filter(s => (status === 'all' || s.status === status) && [s.name,s.objective,s.scope].join(' ').toLocaleLowerCase('pt-BR').includes(query.trim().toLocaleLowerCase('pt-BR'))));
</script>
<section aria-label="Biblioteca de habilidades">
  <h1>Habilidades</h1>
  <p>Procedimentos reutilizáveis, revisados e versionados. Uma habilidade não concede permissões.</p>
  <div class="filters">
    <label>Buscar<input type="search" bind:value={query} /></label>
    <label>Status<select bind:value={status}><option value="all">Todos</option>{#each Object.entries(labels) as [key,label] (key)}<option value={key}>{label}</option>{/each}</select></label>
  </div>
  {#if loading}<p role="status">Carregando habilidades…</p>
  {:else if error}<p role="alert">{error}</p>
  {:else if skills.length === 0}<p>Nenhuma habilidade cadastrada.</p>
  {:else if filtered.length === 0}<p>Nenhuma habilidade corresponde aos filtros.</p>
  {:else}
    <p role="status">{filtered.length} habilidades</p>
    <div class="cards">
      {#each filtered as skill (skill.id)}
        <Card title={skill.name}>
          {#snippet badge()}<Badge tone={skill.status === 'published' ? 'ok' : 'default'}>{labels[skill.status]}</Badge>{/snippet}
          <p>{skill.objective}</p>
          <p>Escopo: {skill.scope} · Versão {skill.version}</p>
          {#if onopen}<Button onclick={() => onopen?.(skill.id)}>Consultar habilidade</Button>{/if}
        </Card>
      {/each}
    </div>
  {/if}
</section>
<style>
  .filters, .cards { display: grid; grid-template-columns: repeat(auto-fit,minmax(min(100%,16rem),1fr)); gap: 1rem; }
  label { display: grid; gap: .35rem; }
  input, select { font: inherit; padding: .65rem; border: 1px solid var(--tina-border); border-radius: var(--tina-radius-lg); background: var(--tina-surface); color: inherit; min-width: 0; }
  input:focus-visible, select:focus-visible { outline: 2px solid var(--tina-pink); }
  p { overflow-wrap: anywhere; }
</style>
