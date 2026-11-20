# TINA-ORG-006 — Relatório de Validação

**Issue:** #6 — Hierarquia dinâmica e histórico de relações  
**Branch:** `feat/tina-org-006-dynamic-hierarchy`  
**Data:** 2026-11-20

---

## 1. Escopo entregue

- Hierarquia dinâmica persistida em `employees.supervisor_id` (fonte de verdade da relação atual).
- Histórico append-only de atribuições, trocas e remoções de superior.
- Histórico de movimentação de cargo/departamento, com cargos e departamentos anteriores e novos.
- Validação no backend Rust: auto-supervisão, superior inexistente/inativo, empresa/instalação e ciclos diretos e indiretos.
- Consultas de cadeia de superiores, subordinados diretos, descendentes com profundidade e árvore por empresa.
- Alteração da relação e registro do histórico atômicos (`BEGIN IMMEDIATE` + transação).
- Cargo e histórico de movimentação atômicos, incluindo transferência entre departamentos.
- Históricos protegidos contra alteração e exclusão por triggers SQLite.
- Comportamento documentado para funcionários inativos.
- 28 testes de integração com SQLite real.

Fora de escopo (não implementado): interface administrativa (issue #9), motor de IA, delegação, chats, novos comandos IPC além do campo aditivo `reason`.

## 2. Arquivos alterados

| Arquivo | Mudança |
|---------|---------|
| `migration/005_employee_supervisor_history.sql` | Tabela `employee_supervisor_history`, índices e FKs |
| `migration/006_employee_assignment_history.sql` | Tabela `employee_assignment_history` + triggers append-only nos dois históricos |
| `src/core/hierarchy.rs` | Tipos de domínio da hierarquia |
| `src/core/assignment.rs` | `AssignmentHistoryEntry` e `PositionChange` |
| `src/core/mod.rs` | Declara e reexporta os módulos |
| `src/infra/hierarchy.rs` | `HierarchyRepository`, `HierarchyService` e `HierarchyError` |
| `src/infra/assignment.rs` | `AssignmentRepository` e `AssignmentService` |
| `src/infra/mod.rs` | Exporta os novos tipos |
| `src/error.rs` | Variante `TinaError::Hierarchy` |
| `src/infra/db.rs` | `busy_timeout` de 5 s no pool SQLite |
| `src/infra/employee.rs` | Criação transacional com evento inicial; delegação de superior e de movimentação aos serviços |
| `src-tauri/src/commands/employee.rs` | Comandos usam os serviços; `reason` opcional em superior e mudança de cargo |
| `src/tests/hierarchy.rs` | 21 testes de integração |
| `src/tests/assignment.rs` | 7 testes de movimentação e imutabilidade |
| `src/tests/mod.rs` | Registra os módulos de teste |
| `docs/architecture/ORGANIZATIONAL-DOMAIN.md` | Modelo, regras, ciclos, históricos, inativos, transações e limitações |
| `docs/reports/TINA-ORG-006-VALIDATION.md` | Este relatório |

## 3. Migrations adicionadas

### 3.1 `005_employee_supervisor_history.sql`

```sql
employee_supervisor_history(
  id TEXT PK,
  employee_id TEXT NOT NULL REFERENCES employees(id) ON DELETE RESTRICT,
  previous_supervisor_id TEXT REFERENCES employees(id) ON DELETE SET NULL,
  new_supervisor_id TEXT REFERENCES employees(id) ON DELETE SET NULL,
  operation TEXT NOT NULL CHECK (operation IN ('assigned','changed','removed')),
  reason TEXT,
  author_id TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
)
```

Índices em `(employee_id, created_at)`, `previous_supervisor_id` e `new_supervisor_id`.

### 3.2 `006_employee_assignment_history.sql`

```sql
employee_assignment_history(
  id TEXT PK,
  employee_id TEXT NOT NULL REFERENCES employees(id) ON DELETE RESTRICT,
  previous_position_id TEXT NOT NULL REFERENCES positions(id) ON DELETE RESTRICT,
  new_position_id TEXT NOT NULL REFERENCES positions(id) ON DELETE RESTRICT,
  previous_department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE RESTRICT,
  new_department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE RESTRICT,
  reason TEXT,
  author_id TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
)
```

Índices por funcionário/data e por departamento anterior/novo. Triggers
`BEFORE UPDATE` e `BEFORE DELETE` com `RAISE(ABORT)` em
`employee_supervisor_history` e `employee_assignment_history` tornam os logs
efetivamente append-only; `INSERT` permanece permitido.

Nenhuma migration anterior foi editada. O upgrade preserva relações, cargos e
dados existentes e não inventa eventos históricos.

## 4. Decisões de modelagem

- **`supervisor_id` preservado** como fonte de verdade do presente: já atendia um superior por subordinado, vários subordinados e profundidade variável.
- **Histórico separado e append-only**, sem espelhar a relação atual em outra tabela.
- **Departamento derivado do cargo:** armazenar apenas `position_id` no funcionário evita divergência; a movimentação grava os departamentos anterior e novo derivados das posições.
- **Ciclo** verificado percorrendo toda a cadeia de ancestrais do superior proposto, via CTE recursiva.
- **Concorrência:** `BEGIN IMMEDIATE` serializa leitura e escrita; `busy_timeout` evita falha imediata sob lock.
- **Inativos:** desativação preserva relação e histórico; funcionário inativo não é aceito como novo superior nem pode ter a relação alterada.
- **Autor:** `author_id` fica `NULL` — não há identidade autenticada por operação, e o proprietário não é atribuído falsamente.
- **No-op:** solicitar o mesmo superior ou o mesmo cargo não grava evento.
- **Imutabilidade:** garantida no banco por triggers, não apenas por convenção de código.

## 5. Testes executados

### 5.1 Hierarquia — `cargo test --lib tests::hierarchy`

| Teste | Resultado |
|-------|-----------|
| `test_valid_relation_created` | ok |
| `test_three_level_hierarchy_and_queries` | ok |
| `test_deep_hierarchy_without_artificial_limit` | ok |
| `test_change_supervisor_records_previous_and_new` | ok |
| `test_remove_relation_preserves_history` | ok |
| `test_full_history_is_chronological` | ok |
| `test_noop_assignment_does_not_create_history` | ok |
| `test_self_supervision_rejected` | ok |
| `test_direct_cycle_rejected` | ok |
| `test_indirect_cycle_rejected` | ok |
| `test_relation_between_companies_rejected` | ok |
| `test_relation_between_installations_rejected` | ok |
| `test_nonexistent_supervisor_rejected` | ok |
| `test_inactive_supervisor_rejected_by_status` | ok |
| `test_failure_updating_relation_leaves_no_partial_state` | ok |
| `test_failure_writing_history_rolls_back_relation` | ok |
| `test_relation_and_history_persist_after_reopen` | ok |
| `test_company_isolation_for_tree_and_queries` | ok |
| `test_deactivated_employee_relation_is_preserved` | ok |
| `test_migrations_preserve_existing_relations` | ok |
| `test_concurrent_changes_cannot_create_cycle` | ok |

**Resultado:** `21 passed; 0 failed; 0 ignored`.

### 5.2 Movimentação e imutabilidade — `cargo test --lib tests::assignment`

| Teste | Resultado |
|-------|-----------|
| `test_change_position_records_history_in_same_department` | ok |
| `test_change_position_records_previous_and_new_departments` | ok |
| `test_same_position_does_not_create_history` | ok |
| `test_failure_writing_assignment_history_rolls_back_position` | ok |
| `test_assignment_history_rejects_update_and_delete` | ok |
| `test_supervisor_history_rejects_update_and_delete` | ok |
| `test_assignment_history_persists_after_reopen` | ok |

**Resultado:** `7 passed; 0 failed; 0 ignored`.

### 5.3 Suíte completa

`cargo test --workspace` → **71 passed; 0 failed** (43 testes anteriores da missão 005 preservados).

## 6. Comandos e resultados

| Comando | Resultado |
|---------|-----------|
| `cargo fmt --all` | executado sem erro |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --workspace` | exit 0 (apenas warnings pré-existentes de dependências não usadas) |
| `cargo test --workspace` | exit 0 — 71 passed, 0 failed |
| `cargo test --lib tests::hierarchy` | exit 0 — 21 passed, 0 failed |
| `cargo test --lib tests::assignment` | exit 0 — 7 passed, 0 failed |

## 7. Evidências de persistência, atomicidade e isolamento

- **Persistência:** `test_relation_and_history_persist_after_reopen` e `test_assignment_history_persists_after_reopen` fecham o pool, reabrem o arquivo SQLite e confirmam relação, cargo e históricos.
- **Atomicidade (hierarquia):** triggers de teste forçam falha no `UPDATE` e no `INSERT` do histórico; em ambos os casos a relação permanece nula e não há evento.
- **Atomicidade (movimentação):** trigger de teste força falha no `INSERT` do histórico; o cargo permanece o anterior e nenhum evento é gravado. Após remover o trigger, a operação legítima funciona.
- **Imutabilidade:** `UPDATE` e `DELETE` diretos em `employee_supervisor_history` e `employee_assignment_history` são rejeitados pelos triggers; o evento original permanece e novas inserções continuam funcionando.
- **Empresa:** `test_company_isolation_for_tree_and_queries` confirma árvore, cadeia e subordinados escopados; `test_change_position_records_previous_and_new_departments` confirma que o histórico de movimentação de outra empresa não é retornado.
- **Instalação:** `test_relation_between_installations_rejected` usa duas instalações e rejeita superior externo.
- **Concorrência:** duas tarefas tentam criar A→B e B→A ao mesmo tempo; exatamente uma vence e o estado final tem uma única aresta.
- **Upgrade:** `test_migrations_preserve_existing_relations` aplica 001→006 sobre banco legado, preserva relação e cargo existentes, não cria eventos fictícios e confirma os quatro triggers de imutabilidade.
- **Departamento:** `test_change_position_records_previous_and_new_departments` verifica cargos e departamentos anterior/novo; o caso de mesmo departamento também é coberto.

## 8. Limitações e testes não executados

- `author_id` permanece `NULL`: não existe autenticação por operação no modelo local atual. Os serviços aceitam autor no nível de backend para uso futuro.
- Limite defensivo de 10.000 níveis nas CTEs — não é limite funcional; protege contra dados legados com ciclo. O teste de profundidade usa 50 níveis.
- A concorrência foi exercitada no processo de teste, não em múltiplos processos do aplicativo desktop.
- Nenhuma validação de janela Tauri real foi executada; o frontend não foi alterado.
- Nenhum novo comando IPC foi exposto para consulta de árvore; as consultas existem no serviço/repositório e a árvore pode ser derivada de `list_employees_by_company` (que retorna `supervisor_id`). A exposição administrativa pertence à issue #9.
- **CI do GitHub:** run final [44829356603](https://github.com/vihisantos/Tina-Gestao-InterPessoal/actions/runs/44829356603) — governance **pass** (5s), Svelte **pass** (22s), Rust/Tauri Windows **pass** (6m10s). A execução do novo SHA desta rodada de ajustes é registrada na PR #15.
