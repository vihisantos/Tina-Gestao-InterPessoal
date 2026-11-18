<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import Card from '$lib/components/Card.svelte';
    import Button from '$lib/components/Button.svelte';
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
    
    onMount(() => loadCompanies());
</script>

<div class="admin-header">
    <h1>Administração da Tina</h1>
    <p class="subtitle">Gestão de empresas, departamentos e cargos</p>
</div>

<Card title="Empresas" badge="total-companies">
    {#if $isLoading}
        <div>Carregando...</div>
    {:else if $error}
        <div class="error">{$error}</div>
    {:else if $companies.length === 0}
        <div class="empty-state">Nenhuma empresa cadastrada.</div>
    {:else}
        <ul class="company-list">
            {#each $companies as company}
                <li class="company-item">
                    <div>
                        <h3>{company.name}</h3>
                        <p class="meta">ID: {company.id.substring(0, 8)}...</p>
                    </div>
                </li>
            {/each}
        </ul>
    {/if}
</Card>

<style>
    .admin-header {
        margin-bottom: var(--tina-spacing-md);
    }
    
    .admin-header h1 {
        margin: 0 0 var(--tina-spacing-xs);
        color: var(--tina-text);
    }
    
    .subtitle {
        color: var(--tina-text-muted);
        margin: 0;
    }
    
    .error {
        color: var(--tina-pink-deep);
        padding: var(--tina-spacing-md);
        text-align: center;
    }
    
    .empty-state {
        color: var(--tina-text-muted);
        font-style: italic;
        padding: var(--tina-spacing-md);
        text-align: center;
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
    
    .company-item h3 {
        margin: 0;
        color: var(--tina-text);
    }
    
    .meta {
        margin: 4px 0 0;
        font-size: 0.85rem;
        color: var(--tina-text-muted);
    }
</style>