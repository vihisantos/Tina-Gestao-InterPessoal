<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import Card from '$lib/components/Card.svelte';
    import Button from '$lib/components/Button.svelte';
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
    const error = writable<string | null>(null);
    
    async function loadCompanies() {
        isLoading.set(true);
        error.set(null);
        try {
            const data = await invoke<CompanyDto[]>('list_companies');
            companies.set(data);
        } catch (e) {
            error.set(e instanceof Error ? e.message : String(e));
        } finally {
            isLoading.set(false);
        }
    }
    
    async function createCompany(name: string) {
        try {
            const newCompany = await invoke<CompanyDto>('create_company', {
                name: name
            });
            const current = companies.get();
            companies.set([...current, newCompany]);
        } catch (e) {
            console.error('Failed to create company:', e);
            alert(e instanceof Error ? e.message : String(e));
        }
    }
    
    async function deleteCompany(id: string) {
        try {
            await invoke('archive_company', { company_id: id });
            const current = companies.get();
            companies.set(current.filter(c => c.id !== id));
        } catch (e) {
            console.error('Failed to archive company:', e);
            alert(e instanceof Error ? e.message : String(e));
        }
    }
    
    onMount(() => loadCompanies());
</script>

<div class="organizational-header">
    <h1>Gestão de Empresas</h1>
    <p class="subtitle">Administre empresas, departamentos e cargos da sua instalação</p>
</div>

{#if $isLoading}
    <div class="loading">Carregando empresas...</div>
{:else if $error}
    <div class="error">{$error}</div>
{:else if $companies.length === 0}
    <p class="empty-state">Nenhuma empresa cadastrada.</p>
{:else}
    <Card title="Empresas" badge="total-companies">
        <ul class="company-list">
            {#each $companies as company}
                <li class="company-item">
                    <div class="company-info">
                        <h3>{company.name}</h3>
                        <p class="meta">ID: {company.id.substring(0, 8)}...</p>
                        <p class="meta">Criado: {new Date(company.created_at).toLocaleDateString('pt-BR')}</p>
                    </div>
                    <div class="company-actions">
                        <Badge tone={company.status === 'active' ? 'ok' : 'default'}>{company.status}</Badge>
                        <button class="btn-delete" on:click={() => deleteCompany(company.id)}>
                            Arquivar
                        </button>
                    </div>
                </li>
            {/each}
        </ul>
    </Card>
{/if}

<style>
    .organizational-header {
        margin-bottom: var(--tina-spacing-md);
    }
    
    .organizational-header h1 {
        margin: 0 0 var(--tina-spacing-xs);
        color: var(--tina-text);
    }
    
    .subtitle {
        color: var(--tina-text-muted);
        margin: 0;
    }
    
    .loading, .error {
        padding: var(--tina-spacing-md);
        text-align: center;
    }
    
    .empty-state {
        color: var(--tina-text-muted);
        font-style: italic;
    }
    
    .company-list {
        list-style: none;
        padding: 0;
        margin: 0;
    }
    
    .company-item {
        padding: var(--tina-spacing-md);
        border-bottom: 1px solid var(--tina-border);
    }
    
    .company-item:last-child {
        border-bottom: none;
    }
    
    .company-info h3 {
        margin: 0;
        color: var(--tina-text);
    }
    
    .meta {
        margin: 0;
        font-size: 0.85rem;
        color: var(--tina-text-muted);
    }
    
    .company-actions {
        display: flex;
        align-items: center;
        gap: var(--tina-spacing-sm);
    }
    
    .btn-delete {
        background: transparent;
        border: 1px solid var(--tina-border-strong);
        color: var(--tina-pink-strong);
        padding: var(--tina-spacing-sm) var(--tina-spacing-md);
        border-radius: var(--tina-radius-sm);
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .btn-delete:hover {
        background: var(--tina-surface-tint);
    }
</style>