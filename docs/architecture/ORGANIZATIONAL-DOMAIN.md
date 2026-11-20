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
| `004_employee_status_sync.sql` | Corrige o backfill da migration 003 (`is_active = 0` → `status = 'inactive'`), sincroniza `is_active` com `status` e recria a view `active_employees` |
| `005_employee_supervisor_history.sql` | Cria o log append-only `employee_supervisor_history` para alterações de superior direto |
| `006_employee_assignment_history.sql` | Cria o log append-only `employee_assignment_history` (cargo/departamento) e triggers que impedem `UPDATE`/`DELETE` nos dois históricos |

**Fonte de verdade do estado do funcionário:** a coluna `status` (`'active'` / `'inactive'`). A coluna `is_active` é mantida por compatibilidade e sempre sincronizada com `status` em todas as operações de criação, desativação e reativação. A view `active_employees` e a validação de superior (`EmployeeRepository::is_active_in_company`) filtram por `status = 'active'`; nenhuma consulta de funcionário ativo usa `is_active`. O `model_config` gravado na criação (`nvidia/nemotron-3.5-bolt-e4b`) é provisório e não define política de modelos.

### 2.7 Tabelas Adicionais

- `delegations` — Tarefas delegadas
- `memories` — Memórias com `company_id`, `employee_id`, `department_id`
- `events` — Auditoria (sem `company_id` na migration)
- `tools` — Ferramentas disponíveis
- `sessions`, `messages` — Conversas
- `config` — Configurações chave-valor

### 2.8 Hierarquia Dinâmica e Histórico (Missão Tina-ORG-006)

#### 2.8.1 Modelo atual e fonte de verdade

A relação **atual** é `employees.supervisor_id`, uma autorreferência com
`ON DELETE SET NULL` e índice `idx_employees_supervisor`. Ela já suporta um
superior por subordinado, vários subordinados por superior e profundidade
variável, então foi preservada como **única fonte de verdade do presente**.
Não existe tabela espelho da relação atual.

O histórico vive em `employee_supervisor_history`, um log **append-only**.
Desde a migration 006, triggers `BEFORE UPDATE` e `BEFORE DELETE` executam
`RAISE(ABORT)` e impedem alterar ou apagar eventos; apenas `INSERT` é aceito.

| Campo | Descrição |
|-------|-----------|

| `employee_id` | Funcionário cuja relação mudou (`ON DELETE RESTRICT`) |
| `previous_supervisor_id` | Superior anterior, quando havia |
| `new_supervisor_id` | Novo superior, quando há |
| `operation` | `assigned`, `changed` ou `removed` |
| `reason` | Motivo opcional informado pelo chamador |
| `author_id` | Autor, quando houver identidade confiável |
| `created_at` | Instante da operação |

`Cargo` e `superior` são independentes: o cargo descreve a posição; o superior
descreve a quem o funcionário responde. A view legada `employee_hierarchy`
exibe `positions.level` como `hierarchy_level`; esse valor é o nível do cargo e
**não** representa profundidade hierárquica — a profundidade correta é
calculada pelas consultas recursivas do serviço.

#### 2.8.2 Regras aplicadas no backend

- Um funcionário não pode ser seu próprio superior.
- O superior deve existir, estar ativo (`status = 'active'`) e pertencer à
  mesma empresa (a empresa já pertence a uma instalação validada no comando).
- Ciclos diretos e indiretos são rejeitados: a alteração é recusada se o
  funcionário aparecer em qualquer ponto da cadeia de ancestrais do superior
  proposto.
- Funcionário inativo não pode ser escolhido como novo superior nem ter sua
  própria relação alterada.
- Toda consulta recebe `company_id` e valida o escopo via
  funcionário → cargo → departamento → empresa.

Erros de regra usam `HierarchyError` (tipado) e são convertidos em
`TinaError::Hierarchy`; o comando Tauri serializa a mensagem.

#### 2.8.3 Detecção de ciclos

`HierarchyRepository::is_in_supervisor_chain` percorre toda a cadeia de
superiores com CTE recursiva (`WITH RECURSIVE`). O limite de 10.000 níveis é
**defensivo**, para não travar em dados legados já corrompidos; não é um limite
funcional da organização.

#### 2.8.4 Consistência transacional e concorrência

`HierarchyService::set_supervisor` executa `BEGIN IMMEDIATE`, que adquire o lock
de escrita antes de ler a cadeia. Assim, duas alterações simultâneas são
serializadas; a segunda enxerga o estado confirmado pela primeira e a validação
de ciclo não pode ser burlada. A atualização de `supervisor_id` e a inserção no
histórico ocorrem na mesma transação: falha em qualquer uma faz `ROLLBACK`.
O pool usa `busy_timeout` de 5 s para esperar o lock em vez de falhar de imediato.

#### 2.8.5 Funcionários inativos

Desativar um funcionário **não** altera `supervisor_id` nem remove histórico.
Relações existentes permanecem visíveis nas consultas com
`include_inactive = true`; listagens de ativos as omitem por `status`.
Reativação restaura a participação com a relação preservada. Reorganizações
devem ser explícitas, por uma nova chamada validada de alteração de superior.

#### 2.8.6 Autor da operação — limitação conhecida

A instalação é local e de usuário único, sem autenticação por operação. Por
isso `author_id` é gravado como `NULL`; o proprietário da instalação **não** é
usado como autor. Quando houver identidade autenticada, o serviço já aceita
`author_id` no nível de backend para preencher o campo sem mudar o schema.

#### 2.8.7 Consultas disponíveis

- `supervisor_chain(employee, company)` — superiores, do direto à raiz.
- `direct_subordinates(supervisor, company, include_inactive)` — filhos diretos.
- `descendants(supervisor, company, include_inactive)` — toda a descendência com profundidade.
- `company_tree(company, include_inactive)` — árvore da empresa; nós inalcançáveis (dados legados com ciclo) aparecem com `depth = None`.
- `history(employee, company)` — eventos em ordem cronológica.

### 2.9 Histórico de Movimentação de Cargo/Departamento

A posição **atual** continua em `employees.position_id`; o departamento é
derivado da cadeia `position → department`. Cada mudança de cargo gera um
evento em `employee_assignment_history`, preservando a trilha de transferências.

| Campo | Descrição |
|-------|-----------|
| `employee_id` | Funcionário movimentado (`ON DELETE RESTRICT`) |
| `previous_position_id` / `new_position_id` | Cargo anterior e novo (`ON DELETE RESTRICT`) |
| `previous_department_id` / `new_department_id` | Departamento derivado do cargo anterior e do novo (`ON DELETE RESTRICT`) |
| `reason` | Motivo opcional informado pelo chamador |
| `author_id` | Autor, quando houver identidade confiável (hoje `NULL`) |
| `created_at` | Instante da operação |

Regras aplicadas:

- O novo cargo deve existir, estar ativo e pertencer a um departamento ativo da
  mesma empresa do funcionário.
- Cargo e histórico são gravados na mesma transação (`BEGIN IMMEDIATE`); falha
  em qualquer etapa faz `ROLLBACK`, sem deixar posição sem evento.
- Solicitar o cargo que o funcionário já ocupa é um **no-op**: nada é gravado e
  `PositionChange::recorded` retorna `false`.
- Mesmo quando o cargo muda dentro do mesmo departamento, o evento é registrado
  com `previous_department_id == new_department_id`.
- `author_id` segue a mesma limitação da hierarquia: `NULL` sem autenticação por
  operação; o serviço aceita autor no backend para uso futuro.
- `AssignmentService::history(employee, company)` retorna os eventos em ordem
  cronológica e valida o escopo da empresa via funcionário → cargo → departamento.
- Triggers da migration 006 rejeitam `UPDATE` e `DELETE`; inserções legítimas
  continuam permitidas.

O comando Tauri `change_employee_position` mantém a superfície existente e
aceita `reason` opcional (campo aditivo), usando o mesmo serviço transacional.

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
| `src/core/hierarchy.rs` | Implementado (Missão 006) | Tipos de operação, evento histórico, resultado de alteração e nó da árvore |
| `src/infra/hierarchy.rs` | Implementado (Missão 006) | `HierarchyRepository` (SQL/transação) e `HierarchyService` (regras, ciclos, histórico, consultas) |
| `src/core/assignment.rs` | Implementado (Missão 006) | Evento de movimentação e resultado da mudança de cargo/departamento |
| `src/infra/assignment.rs` | Implementado (Missão 006) | `AssignmentService`: validação, transação atômica e histórico de movimentações |
| `src/tests/hierarchy.rs` | Implementado (Missão 006) | 21 testes de integração (relações, ciclos, isolamento, rollback, concorrência, persistência, upgrade) |
| `src/tests/assignment.rs` | Implementado (Missão 006) | 7 testes de movimentação, rollback, escopo, persistência e imutabilidade |

### 3.2 src-tauri/ (Tauri 2 integration)

| Arquivo | Status | Evidência |
|---------|--------|-----------|
| `src-tauri/build.rs` | Implementado | Build script Tauri padrão |
| `src-tauri/src/main.rs` | Implementado | Entry point Tauri 2, registra commands de funcionários |
| `src-tauri/src/commands/employee.rs` | Implementado (Missões 005/006) | 10 commands: create, get, list by company/department, update info/responsibilities, change position (com `reason` opcional), assign supervisor, deactivate, reactivate |

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

### 4.2 Concluído — Missão Tina-ORG-006 (Hierarquia Dinâmica e Histórico)

- Hierarquia dinâmica persistida em `employees.supervisor_id` (fonte de verdade atual)
- Histórico append-only em `employee_supervisor_history` (migration 005)
- Ciclos diretos e indiretos bloqueados por CTE recursiva no backend
- Alteração de relação + histórico atômicos com `BEGIN IMMEDIATE`
- Isolamento por empresa/instalação e tratamento de funcionários inativos cobertos por testes
- Histórico de movimentação de cargo/departamento atômico (`employee_assignment_history`, migration 006)
- Históricos protegidos contra `UPDATE`/`DELETE` por triggers (`RAISE(ABORT)`)
- 21 testes de hierarquia (`src/tests/hierarchy.rs`) + 7 de movimentação (`src/tests/assignment.rs`)

### 4.3 Restantes para Fase 2

1. Criar auditoria de eventos organizacionais
2. Implementar interface administrativa básica
3. Expor consultas de árvore na UI (issue #9)