<script lang="ts">
  import Card from './Card.svelte';
  import Badge from './Badge.svelte';
  import Button from './Button.svelte';

  let { name, position, department, company, active, supervisor, responsibilities, onopen }: {
    name: string;
    position: string;
    department: string;
    company: string;
    active: boolean;
    supervisor?: string;
    responsibilities?: string;
    onopen?: () => void;
  } = $props();

  let initials = $derived(name.trim().split(/\s+/).filter(Boolean).slice(0, 2).map(part => Array.from(part)[0]).join('').toLocaleUpperCase('pt-BR'));
</script>

<Card title="Perfil do funcionário">
  {#snippet badge()}
    <Badge tone={active ? 'ok' : 'default'}>{active ? 'Ativo' : 'Inativo'}</Badge>
  {/snippet}
  <div class="identity">
    <span class="avatar" aria-hidden="true">{initials || '—'}</span>
    <div class="identity-text">
      <h3>{name}</h3>
      <p>{position}</p>
    </div>
  </div>
  <dl>
    <div><dt>Empresa</dt><dd>{company}</dd></div>
    <div><dt>Departamento</dt><dd>{department}</dd></div>
    {#if supervisor}<div><dt>Supervisor</dt><dd>{supervisor}</dd></div>{/if}
    {#if responsibilities}<div><dt>Responsabilidades</dt><dd>{responsibilities}</dd></div>{/if}
  </dl>
  {#if onopen}<Button onclick={onopen}>Ver perfil de {name}</Button>{/if}
</Card>

<style>
  .identity { display: flex; align-items: center; gap: .85rem; }
  .avatar { display: grid; place-items: center; width: 3rem; height: 3rem; flex: 0 0 3rem; border-radius: var(--tina-radius-pill); background: var(--tina-surface-pink); color: var(--tina-pink-deep); font-weight: 700; }
  .identity-text { min-width: 0; }
  h3, p, dl, dd { margin: 0; overflow-wrap: anywhere; }
  h3 { font-size: 1.1rem; }
  p, dt { font-size: .85rem; }
  dl { display: grid; gap: .65rem; }
  dt { font-weight: 600; }
  dd { white-space: pre-wrap; }
</style>
