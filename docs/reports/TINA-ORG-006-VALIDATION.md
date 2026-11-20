# TINA-ORG-006 — Relatório de Validação

**Issue:** #6 — Hierarquia dinâmica e histórico de relações  
**Branch:** `feat/tina-org-006-dynamic-hierarchy`  
**Data:** 2026-11-20

---

## 1. Escopo entregue

- Hierarquia dinâmica persistida em `employees.supervisor_id` (fonte de verdade da relação atual).
- Histórico append-only de atribuições, trocas e remoções de superior.
- Validação no backend Rust: auto-supervisão, superior inexistente/inativo, empresa/instalação e ciclos diretos e indiretos.
- Consultas de cadeia de superiores, subordinados diretos, descendentes com profundidade e árvore por empresa.
- Alteração da relação e registro do histórico atômicos (`BEGIN IMMEDIATE` + transação).
- Comportamento documentado para funcionários inativos.
- 21 testes de integração com SQLite real.

Fora de escopo (não implementado): interface administrativa (issue #9), motor de IA, delegação, chats, novos comandos IPC.

## 2. Arquivos alterados

| Arquivo | Mudança |
|---------|---------|
| `migration/005_employee_supervisor_history.sql` | Nova tabela `employee_supervisor_history`, índices e FKs |
| `src/core/hierarchy.rs` | Novos tipos de domínio (`HierarchyOperation`, `SupervisorHistoryEntry`, `SupervisorChange`, `HierarchyNode`) |
| `src/core/mod.rs` | Declara e reexporta o módulo de hierarquia |
| `src/infra/hierarchy.rs` | `HierarchyRepository` (SQL/transação), `HierarchyService` (regras, ciclos, consultas) e `HierarchyError` |
| `src/infra/mod.rs` | Exporta os novos tipos |
| `src/error.rs` | Variante `TinaError::Hierarchy` |
| `src/infra/db.rs` | `busy_timeout` de 5 s no pool SQLite |
| `src/infra/employee.rs` | Criação transacional com evento inicial de histórico; `assign_supervisor` delega ao serviço |
| `src-tauri/src/commands/employee.rs` | `assign_employee_supervisor` usa o serviço; campo opcional `reason` |
| `src/tests/hierarchy.rs` | 21 testes de integração |
| `src/tests/mod.rs` | Registra o módulo de testes |
| `docs/architecture/ORGANIZATIONAL-DOMAIN.md` | Modelo, regras, ciclos, histórico, inativos, transações e limitações |
| `docs/reports/TINA-ORG-006-VALIDATION.md` | Este relatório |

## 3. Migration adicionada

`migration/005_employee_supervisor_history.sql` — cria:

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

Nenhuma migration anterior foi editada. Não há exclusão destrutiva nem perda de dados: a relação atual permanece intacta e o upgrade não inventa eventos históricos.

## 4. Decisões de modelagem

- **`supervisor_id` preservado** como fonte de verdade do presente: já atendia um superior por subordinado, vários subordinados e profundidade variável.
- **Histórico separado e append-only**, sem espelhar a relação atual em outra tabela.
- **Ciclo** verificado percorrendo toda a cadeia de ancestrais do superior proposto, via CTE recursiva.
- **Concorrência:** `BEGIN IMMEDIATE` serializa leitura da cadeia e escrita; `busy_timeout` evita falha imediata sob lock.
- **Inativos:** desativação preserva relação e histórico; funcionário inativo não é aceito como novo superior nem pode ter a relação alterada.
- **Autor:** `author_id` fica `NULL` — não há identidade autenticada por operação, e o proprietário não é atribuído falsamente.

## 5. Testes executados

Comando: `cargo test --lib tests::hierarchy`

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
| `test_migration_005_preserves_existing_relations` | ok |
| `test_concurrent_changes_cannot_create_cycle` | ok |

**Resultado:** `21 passed; 0 failed; 0 ignored`.

Suíte completa: `cargo test --workspace` → **64 passed; 0 failed** (43 testes anteriores preservados).

## 6. Comandos e resultados

| Comando | Resultado |
|---------|-----------|
| `cargo fmt --all` | executado sem erro |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --workspace` | exit 0 (apenas warnings pré-existentes de dependências não usadas) |
| `cargo test --workspace` | exit 0 — 64 passed, 0 failed |
| `cargo test --lib tests::hierarchy` | exit 0 — 21 passed, 0 failed |

## 7. Evidências de persistência e isolamento

- **Persistência:** `test_relation_and_history_persist_after_reopen` fecha o pool, reabre o arquivo SQLite e confirma `supervisor_id` e histórico.
- **Atomicidade:** triggers de teste forçam falha no `UPDATE` e no `INSERT` do histórico; em ambos os casos a relação permanece nula e não há evento.
- **Empresa:** `test_company_isolation_for_tree_and_queries` confirma que árvore, cadeia e subordinados de uma empresa não retornam funcionários de outra.
- **Instalação:** `test_relation_between_installations_rejected` usa duas instalações e rejeita superior externo.
- **Concorrência:** duas tarefas tentam criar A→B e B→A ao mesmo tempo; exatamente uma vence e o estado final tem uma única aresta.
- **Upgrade:** `test_migration_005_preserves_existing_relations` aplica 001→005 sobre banco legado, preserva a relação existente, não cria histórico fictício e permite novas alterações.

## 8. Limitações e testes não executados

- `author_id` permanece `NULL`: não existe autenticação por operação no modelo local atual. O serviço aceita autor no nível de backend para uso futuro.
- Limite defensivo de 10.000 níveis nas CTEs — não é limite funcional; protege contra dados legados com ciclo. O teste de profundidade usa 50 níveis.
- A concorrência foi exercitada no processo de teste, não em múltiplos processos do aplicativo desktop.
- Nenhuma validação de janela Tauri real foi executada; o frontend não foi alterado.
- Nenhum novo comando IPC foi exposto para consulta de árvore; as consultas existem no serviço/repositório e a árvore pode ser derivada de `list_employees_by_company` (que retorna `supervisor_id`). A exposição administrativa pertence à issue #9.
- **CI do GitHub (run [44827750342](https://github.com/vihisantos/Tina-Gestao-InterPessoal/actions/runs/44827750342)):** governance checks **pass** (5s), Svelte frontend validation **pass** (15s), Rust and Tauri validation (Windows) **pass** (10m45s — fmt, check e testes).
