<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import Card from '$lib/components/Card.svelte';
    import Badge from '$lib/components/Badge.svelte';
    import { writable } from 'svelte/store';

    interface CompanyDto {
        id: string;
        installation_id: string;
        name: string;
        status: string;
        created_at: string;
    }

    const companies = writable<CompanyDto[]>([]);
    const isLoading = writable(true);
    const isCreating = writable(false);
    const error = writable<string | null>(null);
    let newCompanyName = '';

    async function loadCompanies() {
        isLoading.set(true);
        error.set(null);
        try {
            companies.set(await invoke<CompanyDto[]>('list_companies'));
        } catch (e) {
            error.set(e instanceof Error ? e.message : String(e));
        } finally {
            isLoading.set(false);
        }
    }

    async function createCompany(event: SubmitEvent) {
        event.preventDefault();
        const name = newCompanyName.trim();
        if (!name || isCreating) return;

        isCreating.set(true);
        error.set(null);
        try {
            // O handler Rust recebe CreateCompanyRequest no argumento "request".
            await invoke<CompanyDto>('create_company', { request: { name } });
            newCompanyName = '';
            await loadCompanies();
        } catch (e) {
            error.set(e instanceof Error ? e.message : String(e));
        } finally {
            isCreating.set(false);
        }
    }

    async function archiveCompany(id: string) {
        if (!window.confirm('Deseja realmente arquivar esta empresa?')) return;
        error.set(null);
        try {
            await invoke('archive_company', { company_id: id });
            await loadCompanies();
        } catch (e) {
            error.set(e instanceof Error ? e.message : String(e));
        }
    }

    onMount(() => loadCompanies());
</script>

<div class="organizational-header">
    <h1>Gestão de Empresas</h1>
    <p class="subtitle">Administre empresas, departamentos e cargos da sua instalação</p>
</div>

<Card title="Cadastrar empresa">
    <form class="create-form" onsubmit={createCompany}>
        <label for="company-name">Nome da empresa</label>
        <input id="company-name" name="company-name" bind:value={newCompanyName} maxlength="160" required />
        <button type="submit" disabled={$isCreating || !newCompanyName.trim()}>
            {$isCreating ? 'Cadastrando...' : 'Cadastrar empresa'}
        </button>
    </form>
</Card>

{#if $error}
    <div class="error" role="alert">{$error}</div>
{/if}

{#if $isLoading}
    <div class="loading">Carregando empresas...</div>
{:else if $companies.length === 0}
    <p class="empty-state">Nenhuma empresa cadastrada.</p>
{:else}
    <Card title="Empresas">
        <p class="meta">{$companies.length} empresa(s)</p>
        <ul class="company-list">
            {#each $companies as company (company.id)}
                <li class="company-item">
                    <div class="company-info">
                        <h3>{company.name}</h3>
                        <p class="meta">ID: {company.id.substring(0, 8)}...</p>
                        <p class="meta">Criada: {new Date(company.created_at).toLocaleDateString('pt-BR')}</p>
                    </div>
                    <div class="company-actions">
                        <Badge tone={company.status === 'active' ? 'ok' : 'default'}>{company.status}</Badge>
                        <button class="btn-delete" type="button" onclick={() => archiveCompany(company.id)}>
                            Arquivar
                        </button>
                    </div>
                </li>
            {/each}
        </ul>
    </Card>
{/if}

<style>
    .organizational-header { margin-bottom: var(--tina-spacing-md); }
    .organizational-header h1 { margin: 0 0 var(--tina-spacing-xs); color: var(--tina-text); }
    .subtitle { color: var(--tina-text-muted); margin: 0; }
    .loading, .error { padding: var(--tina-spacing-md); text-align: center; }
    .error { color: var(--tina-pink-deep); }
    .empty-state { color: var(--tina-text-muted); font-style: italic; }
    .create-form { display: flex; flex-wrap: wrap; align-items: end; gap: var(--tina-spacing-sm); }
    .create-form label { flex-basis: 100%; color: var(--tina-text-muted); }
    .create-form input { flex: 1; min-width: 220px; padding: var(--tina-spacing-sm); border: 1px solid var(--tina-border); border-radius: var(--tina-radius-sm); }
    .create-form button { padding: var(--tina-spacing-sm) var(--tina-spacing-md); border: 1px solid var(--tina-border-strong); border-radius: var(--tina-radius-sm); cursor: pointer; }
    .create-form button:disabled { opacity: 0.6; cursor: not-allowed; }
    .company-list { list-style: none; padding: 0; margin: 0; }
    .company-item { display: flex; justify-content: space-between; align-items: center; gap: var(--tina-spacing-md); padding: var(--tina-spacing-md); border-bottom: 1px solid var(--tina-border); }
    .company-item:last-child { border-bottom: none; }
    .company-info h3 { margin: 0; color: var(--tina-text); }
    .meta { margin: 4px 0 0; font-size: 0.85rem; color: var(--tina-text-muted); }
    .company-actions { display: flex; align-items: center; gap: var(--tina-spacing-sm); }
    .btn-delete { background: transparent; border: 1px solid var(--tina-border-strong); color: var(--tina-pink-strong); padding: var(--tina-spacing-sm) var(--tina-spacing-md); border-radius: var(--tina-radius-sm); cursor: pointer; }
    .btn-delete:hover { background: var(--tina-surface-tint); }
</style>
