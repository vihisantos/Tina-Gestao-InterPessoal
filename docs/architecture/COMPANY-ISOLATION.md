# Isolamento entre Empresas — Tina Platform

**Status:** Especificação | **Fase:** 2 — Fundação Organizacional  
**Issue:** #1 — Auditoria técnica e arquitetura do domínio organizacional

---

## 1. Princípio do Isolamento

**Nenhuma operação pode acessar ou modificar dados de outra empresa, mesmo que o frontend envie um `company_id` diferente.**

O isolamento deve ser garantido pelo **backend Rust**, não por:
- Esquemas SQL separados
- Vistas dinâmicas
- Chamadas de API com IDs enviados pelo cliente

---

## 2. Contexto Confiável

### 2.1 Fonte de Autoridade

O contexto da empresa ativa deve ser determinado a partir de:

1. **Seleção explícita do proprietário** (via UI)
2. **Validação no backend** — O backend confia no ID da empresa apenas após validar que:
   - A empresa pertence à mesma `installation_id`
   - O funcionário tem permissão para a empresa
   - A operação é permitida pelo modelo de autoridade

---

## 3. Camadas de Isolamento — Estrutura Real

### 3.1 Departamentos

**FK:** `departments.company_id → companies.id`

```sql
-- Um departamento só existe em uma empresa
SELECT * FROM departments WHERE id = 'D123'  -- Retorna company_id
```

### 3.2 Cargos (Positions)

**FK:** `positions.department_id → departments.id`

**Isolamento transitivo:**
- Posição → Departamento → Empresa

### 3.3 Funcionários (Employees)

**IMPORTANTE:** A migration **NÃO** possui `employees.company_id`.

A empresa de um funcionário deve ser obtida pela relação:

```
employees.position_id
    → positions.department_id
    → departments.company_id
```

**Exemplo de consulta correta:**

```rust
// Pseudocódigo conceitual
async fn get_employee_company(employee_id: &str) -> Result<String, Error> {
    let company_id = sqlx::query_scalar!(
        "SELECT c.id FROM employees e
         JOIN positions p ON e.position_id = p.id
         JOIN departments d ON p.department_id = d.id
         JOIN companies c ON d.company_id = c.id
         WHERE e.id = ?", employee_id
    ).fetch_optional(&pool).await?;

    company_id.ok_or(EmployeeNotFound)?
}
```

**Regras de isolamento:**

| Operação | Validação |
|----------|-----------|
| Listar funcionários | WHERE via JOIN: employees → positions → departments → companies |
| Buscar funcionário específico | Verificar cadeia position → department → company |
| Transferir funcionário | Verificar novo departamento → nova empresa |
| Definir supervisor | Verificar supervisor pertence à mesma empresa (via cadeia acima) |

---

## 4. Problema de `memories.company_id`

A tabela `memories` possui:

```sql
CREATE TABLE memories (
    ...
    employee_id REFERENCES employees(id),
    company_id REFERENCES companies(id),
    department_id REFERENCES departments(id),
    ...
);
```

### 4.1 FK vs. Regras de Negócio

SQLite com `PRAGMA foreign_keys = ON` aplica **apenas** as restrições de integridade referencial declaradas.

**O que as FK garantem:**
- `memories.company_id` aponta para empresa válida
- `memories.employee_id` aponta para funcionário válido
- `memories.department_id` aponta para departamento válido

**O que as FK NÃO garantem:**
- Funcionário e empresa na `memories` são consistentes (FKs isoladas)
- Supervisor e funcionário pertencem à mesma empresa
- Memória e funcionário pertencem à mesma empresa (FKs existem, mas não impõem restrição cruzada)

### 4.2 Validação no Backend

```rust
// Pseudocódigo conceitual
async fn validate_memory_access(
    identity: &Identity,
    memory_company_id: &str,
    memory_employee_id: &str,
) -> Result<bool, Error> {
    // 1. Verificar employee pertence a company
    let emp_company = get_employee_company(memory_employee_id).await?;
    if emp_company != memory_company_id {
        return Ok(false);
    }

    // 2. Verificar política de acesso
    // ...
}
```

---

## 5. Eventos e Auditoria Histórica

### 5.1 Problema Identificado

A tabela `events` **não está vinculada a empresas**, o que dificulta a auditoria por empresa.

### 5.2 Risco

Quando um funcionário é transferido de empresa:
- Eventos históricos não têm contexto empresarial
- Não é possível auditoria precisa da atividade na empresa original

### 5.3 Decisão Arquitetural Recomendada

Para preservar o histórico, **recomenda-se**:

```sql
-- Coluna snapshot (sem FK) para preservar histórico
ALTER TABLE events ADD COLUMN company_id_snapshot TEXT;

-- Ou, alternativa:
ALTER TABLE events ADD COLUMN company_id TEXT REFERENCES companies(id) ON DELETE SET NULL;
ALTER TABLE events ADD COLUMN company_snapshot TEXT NOT NULL;
```

**Estratégia preferida (sem modificar migration agora):**

Quando a migration for evoluída:
- `company_id_snapshot` — armazena o UUID da empresa no momento do evento
- Sem FK → empresas excluídas não apagam eventos históricos
- O valor é **imutável** após inserção

---

## 6. Testes de Isolamento (a implementar)

| Teste | Descrição | Resultado Esperado |
|-------|-----------|-------------------|
| T1 | Funcionário A tenta listar dados de empresa B | ❌ AuthorizationError |
| T2 | Funcionário A tenta criar departamento em empresa C | ❌ AuthorizationError |
| T3 | Funcionário A tenta transferir para empresa B (sem permissão) | ❌ AuthorizationError |
| T4 | Proprietário lista funcionários de todas as empresas | ✅ OK |
| T5 | Supervisor lista subordinate da mesma empresa | ✅ OK |

---

## 7. Decisão Final

- **Nenhum ID de empresa é confiável por si só**
- **Validação sempre no backend Rust**
- **FKs existem, mas não substituem validação de negócio**
- **Proprietário é único e tem acesso total**
- **employees.company_id ** NÃO existe** — usar cadeia de relações
- **Eventos precisam de company_id_snapshot para auditoria completa**