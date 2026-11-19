# DOMínio Organizacional — Tina Platform

**Status:** Especificação | **Fase:** 2 — Fundação Organizacional  
**Issue:** #1 — Auditoria técnica e arquitetura do domínio organizacional  
**Criado em:** 2026-11-17  
**Última atualização:** Esta fase

---

## 1. Visão Geral

O domínio organizacional da Tina representa a estrutura hierárquica e administrativa de múltiplas empresas gerenciadas por uma única instalação local.

---

## 2. Entidades Definidas (Migration 001)

### 2.1 Installation

**Tabela:** `installations`

| Campo | Tipo | Descrição |
|-------|------|-----------|
| `id` | TEXT PK | UUID gerado |
| `name` | TEXT NN | Nome da instalação |
| `root_path` | TEXT NN UK | Caminho raiz |
| `version` | TEXT NN | Versão da instalação |
| `created_at` | TEXT NN | Timestamp de criação |
| `owner_uuid` | TEXT NN | UUID do proprietário |
| `config` | TEXT | Configuração JSON |

### 2.2 Company

**Tabela:** `companies`

| Campo | FK | Índice |
|-------|-----|--------|
| `id` | - | PK |
| `installation_id` | installations → CASCADE | idx_companies_installation |
| `name` | - | - |
| `status` | - | - |
| `created_at` | - | - |

### 2.3 Department

**Tabela:** `departments`

| Campo | FK | Índice |
|-------|-----|--------|
| `id` | - | PK |
| `company_id` | companies → CASCADE | idx_departments_company, idx_departments_code |
| `name` | - | - |
| `code` | - | - |
| `description` | - | - |
| `created_at` | - | - |

### 2.4 Position

**Tabela:** `positions`

| Campo | FK | Índice |
|-------|-----|--------|
| `id` | - | PK |
| `department_id` | departments → CASCADE | idx_positions_department, idx_positions_level, idx_positions_auth |
| `name` | - | - |
| `level` | - | idx_positions_level |
| `authority_level` | - | idx_positions_auth |
| `description` | - | - |
| `created_at` | - | - |

### 2.5 Employee

**Tabela:** `employees` (NÃO tem `company_id`)

| Campo | Tipo | FK | Observação |
|-------|------|-----|------------|
| `id` | TEXT PK | - | UUID gerado |
| `position_id` | TEXT NN | positions → CASCADE | Via cadeia: position → dept → company |
| `name` | TEXT NN | - | Nome/identidade do funcionário |
| `responsibilities` | TEXT | - | Responsabilidades do funcionário |
| `supervisor_id` | TEXT | employees → SET NULL | Superior direto (auto-relação) |
| `model_config` | TEXT NN | - | Configuração do modelo de IA |
| `authority_override` | INTEGER | - | NÃO é permissão automática |
| `status` | TEXT NN | - | Fonte de verdade: 'active' ou 'inactive' (Mig 003) |
| `is_active` | INTEGER NN | - | Mantida por compatibilidade, sincronizada com `status` (1 ou 0) |
| `created_at` | TEXT NN | - | Timestamp de criação |
| `updated_at` | TEXT NN | - | Timestamp de atualização (trigger) |
| `deactivated_at` | TEXT | - | Data de desativação (Mig 002) |

**Importante:** Empresa = `position.department.company`. Não duplicar `company_id` diretamente na tabela.

### 2.6 Migrations Adicionais (Fase 2 - Funcionários)

| Migration | Descrição |
|-----------|-----------|
| `002_employee_deactivated_at.sql` | Adiciona coluna `deactivated_at` à tabela `employees` |
| `003_employee_status_column.sql` | Adiciona coluna `status` à tabela `employees` (consistente com outras tabelas organizacionais) |
| `004_employee_status_sync.sql` | Backfill de `status` a partir de `is_active` e recria a view `active_employees` usando `status` como fonte de verdade |

**Fonte de verdade do estado do funcionário:** a coluna `status` (`'active'` / `'inactive'`). A coluna `is_active` é mantida por compatibilidade e sempre sincronizada com `status` em todas as operações de criação, desativação e reativação. A view `active_employees` filtra por `status = 'active'`.

### 2.7 Tabelas Adicionais

- `delegations` — Tarefas delegadas
- `memories` — Memórias com `company_id`, `employee_id`, `department_id`
- `events` — Auditoria (sem `company_id` na migration)
- `tools` — Ferramentas disponíveis
- `sessions`, `messages` — Conversas
- `config` — Configurações chave-valor

---

## 3. Módulos Rust Auditados

### 3.1 Implementado e Integrado

| Arquivo | Status | Evidência |
|---------|--------|-----------|
| `src/config.rs` | Implementado | `AppConfig`, `load_config()`, `Default` impl — usado em `TinaApp::initialize()` |
| `src/error.rs` | Implementado | `TinaError` enum completo com 18 variants — exportado publicamente |
| `src/lib.rs` | Implementado | Entry point biblioteca, declara módulos principais |
| `src/infra/db.rs` | Implementado | `Database::open()` com SQLx migrations — funcional |
| `src/core/employee.rs` | Implementado (Missão 005) | Entidade `Employee` com `EmployeeStatus`, `is_active()`, `supervisor_id()` |
| `src/infra/employee.rs` | Implementado (Missão 005) | `EmployeeRepository` com CRUD, isolamento por empresa/instalação, desativação/reativação, mudança de cargo, atribuição de supervisor |
| `src/tests/employee.rs` | Implementado (Missão 005) | 20 testes de integração com SQLite isolado |

### 3.2 src-tauri/ (Tauri 2 integration)

| Arquivo | Status | Evidência |
|---------|--------|-----------|
| `src-tauri/build.rs` | Implementado | Build script Tauri padrão |
| `src-tauri/src/main.rs` | Implementado | Entry point Tauri 2, registra commands de funcionários |
| `src-tauri/src/commands/employee.rs` | Implementado (Missão 005) | 10 commands: create, get, list by company/department, update info/responsibilities, change position, assign supervisor, deactivate, reactivate |

### 3.3 Placeholder (Estrutura, não compila ou usado)

| Arquivo | Status | Observação |
|---------|--------|------------|
| `src/ctx.rs` | Placeholder | Apenas comentário: `//! Context module placeholder` |
| `src/core/tina.rs` | Placeholder | Usa `uuid` crate não importado |
| `src/core/scheduler.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/agent/employee.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/agent/hierarchy.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/agent/position.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/agent/worker.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/agent/role.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/agent/manager.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/auth/identity.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/auth/session.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/auth/permission.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/auth/auth_service.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/context/selector.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/context/distributor.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/context/consolidator.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/memory/memory_service.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/memory/organizational.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/memory/personal.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/memory/professional.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/memory/embedding.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/memory/vector_store.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/planner/planner.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/planner/decomposition.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/planner/dependency.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/route/router.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/route/nvidia.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/route/provider.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/route/selector.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/exec/executor.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/exec/sandbox.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/exec/tool_registry.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/eval/evaluator.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/eval/validator.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/adapter/mcp.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/adapter/nvidia_nim.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/activity/event_bus.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/activity/event_store.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/activity/tracker.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/mcp/client.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/mcp/server.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/infra/redis.rs` | Placeholder | Não declarado em `lib.rs` |
| `src/infra/telemetry.rs` | Placeholder | Não declarado em `lib.rs` |

### 3.4 Arquivos Raiz

| Arquivo | Status | Observação |
|---------|--------|------------|
| `src/ctx.rs` | Placeholder | Apenas comentário: `//! Context module placeholder` |

### 3.5 Não Integrados (sem .mod.rs válido)

| Diretório | Status |
|-----------|--------|
| `src/migration` | Não integrado - Vazio |
| `src/tests` | Não integrado - Vazio |

---

## 4. Próximos Passos para Fase 2

### 4.1 Concluído — Missão Tina-ORG-005 (Cadastro e Ciclo de Vida de Funcionários IA)

- Entidade `Employee` implementada (`src/core/employee.rs`)
- Repositório `EmployeeRepository` implementado (`src/infra/employee.rs`)
- Commands Tauri implementados (`src-tauri/src/commands/employee.rs`)
- Migrations 002 e 003 criadas (`migration/002_employee_deactivated_at.sql`, `migration/003_employee_status_column.sql`)
- 20 testes de integração passando (`src/tests/employee.rs`)
- Isolamento entre empresas e instalações comprovado

### 4.2 Restantes para Fase 2

1. Implementar validação de ciclos hierárquicos (impedir supervisor circular)
2. Criar auditoria de eventos organizacionais
3. Implementar interface administrativa básica