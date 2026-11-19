# Modelo de Autoridade — Tina Platform

**Status:** Especificação | **Fase:** 2 — Fundação Organizacional  
**Issue:** #1 — Auditoria técnica e arquitetura do domínio organizacional

> **Estado real da implementação nesta entrega:** o banco e o arquivo local de configuração mantêm `installation_id` e `owner_uuid`, e a inicialização confere se ambos correspondem. Isso estabelece uma identidade local persistida; **não implementa login, sessão autenticada ou autorização de proprietário**. Os comandos Tauri desta fase operam no contexto local do aplicativo. As regras abaixo descrevem o modelo-alvo e não devem ser interpretadas como funcionalidades já entregues.

---

## 1. Princípio Fundamental

**O proprietário (owner) é a autoridade máxima absoluta da instalação.**

Nenhum outro ator — nem mesmo Tina Central, Diretores ou Gerentes — pode:
- Revogar a autoridade do proprietário
- Bloquear o acesso do proprietário a qualquer funcionário
- Impedir que o proprietário consulte informações autorizadas
- Alterar permissões para ultrapassar o proprietário
- Criar hierarquia superior ao proprietário

---

## 2. Identidade, Autenticação e Autorização do Proprietário

### 2.1 Identidade

O proprietário é identificado por `installations.owner_uuid` (coluna TEXT).

**Decisão Arquitetural:** Não há tabela `owners` separada. O proprietário é:
- Representado por um UUID único
- Persistido na tabela `installations`
- Mantido também na configuração local para identificar a instalação
- Comparado com o registro persistido durante a inicialização; divergências interrompem o bootstrap
- **Não é uma credencial e, isoladamente, não prova que a pessoa usando o aplicativo é o proprietário**

### 2.2 Autenticação — ainda não implementada

O frontend **não deve** enviar o UUID do proprietário como prova de autenticação.

**Nesta entrega não existe login, sessão autenticada, token de proprietário ou resolução de identidade autenticada.** A presença de `owner_uuid` na configuração e no banco não autentica o usuário. Um mecanismo local de autenticação deverá ser especificado e implementado em uma fase própria antes de afirmar que operações estão protegidas por identidade autenticada.

O modelo futuro deverá:
1. Receber uma identidade autenticada por mecanismo local seguro
2. Resolver essa identidade no backend
3. Validar permissões com base na identidade resolvida, nunca em um UUID arbitrário enviado pela interface

### 2.3 Autorização — modelo-alvo, não entregue nesta fase

A autorização deverá ser determinada pelo backend **com base em quem está executando a operação**, não por IDs passivamente recebidos do frontend.

**Regra crítica para a implementação futura:** IDs recebidos do frontend são **seletores** que precisam ser validados.

Os comandos organizacionais entregues nesta fase aplicam o escopo de instalação aos dados consultados e alterados, mas ainda não constituem um sistema completo de autenticação/autorização do proprietário ou de funcionários.

## 3. Proprietário vs Funcionário

### 3.1 O Proprietário NÃO é um funcionário

O proprietário:
- **Não está na tabela `employees`**
- **Não tem `position_id`**
- **Não pode ser "desativado"**
- **Não faz parte de relações hierárquicas**
- **Não é identificado por `employee_id`**

### 3.2 Diferenciação Clara

| Aspecto | Proprietário | Funcionário |
|---------|--------------|------------- |
| Identificador | `owner_uuid` em `installations` | `id` em `employees` |
| Tabela | `installations` | `employees` |
| Busca no DB | `SELECT ... FROM installations WHERE owner_uuid = ?` | `SELECT ... FROM employees WHERE id = ?` |
| Acesso total | Sim (hardcodeado) | Condicional (hierarquia + authority_level) |

### 3.3 Pseudocódigo de Implementação

```rust
// Pseudocódigo conceitual
impl TinaApp {
    /// Verifica se a identidade autenticada é o proprietário
    fn is_owner_identity(&self, identity: &Identity) -> bool {
        // A identidade vem de um mecanismo de autenticação seguro,
        // como session token validado ou API key autorizada
        identity.is_owner
    }

    /// Busca empresa de um funcionário via cadeia de relações
    async fn get_employee_company(&self, employee_id: &str) -> Result<Option<String>, Error> {
        sqlx::query_scalar!(
            "SELECT c.id FROM employees e
             JOIN positions p ON e.position_id = p.id
             JOIN departments d ON p.department_id = d.id
             JOIN companies c ON d.company_id = c.id
             WHERE e.id = ?", employee_id
        ).fetch_optional(&self.pool).await
    }
}
```

---

## 4. Hierarquia de Autoridade

```
┌─────────────────────────────────────────────────────────┐
│                    PROPRIETÁRIO (Owner)                 │
│  - UUID da instalação                                   │
│  - ACESSO TOTAL: qualquer funcionário via employee_id    │
│  - Pode criar/alterar/excluir empresas, depts, cargos   │
│  - Não é funcionário (não está em employees table)       │
└────────────────────────┬────────────────────────────────┘
                         │
            ┌────────────┴────────────┐
            ▼                         ▼
┌───────────────────────┐   ┌───────────────────────┐
│   TINA CENTRAL        │   │     DIRETOR(S)        │
│ (Coordenador)         │   │ (authority_level > 0) │
│ - Coordena agentes    │   │ - Pode gerenciar      │
│ - NÃO pode            │   │   departamentos/cargos│
│   ultrapassar owner   │   │ - Limites definidos   │
└───────────────────────┘   └───────────────────────┘
            │                         │
            ▼                         ▼
┌───────────────────────────────────────────────────────┐
│              CHEFE(S) → ANALISTA(S)                   │
│              (Nível hierárquico variável)             │
│  - Supervisionam analistas menores                    │
│  - Reportam a Diretores ou Chefe                      │
└───────────────────────────────────────────────────────┘
```

---

## 5. `authority_override` NÃO é Acesso Ilimitar

### 5.1 Declaração

**O campo `authority_override INTEGER` em `employees` NÃO concede acesso ilimitado automaticamente.**

### 5.2 Política Explícita

TODO: Toda verificação de permissão deve passar por uma política explícita no backend.

### 5.3 Importância

- `authority_override = 1` indica **elegibilidade** a certas operações
- A **autorização real** depende de regras de negócio
- Nenhum funcionário pode ultrapassar o proprietário

---

## 6. Testes de Autoridade (a implementar)

| Teste | Descrição | Resultado Esperado |
|-------|-----------|-------------------|
| T1 | Proprietário acessa funcionário de qualquer empresa | ✅ OK |
| T2 | Funcionário tenta criar relação superior a si mesmo | ❌ AuthorizationError |
| T3 | Funcionário tenta transferir para empresa sem permissão | ❌ AuthorizationError |
| T4 | Funcionário com authority_override tenta acessar empresa B | ❌ AuthorizationError (policy explícita) |
| T5 | Nenhum funcionário pode alterar owner_uuid | ❌ AuthorizationError |

---

## 7. Decisão Final e estado de entrega

- **owner_uuid** identifica o proprietário declarado no registro da instalação; não é credencial de autenticação
- A conferência entre configuração e banco detecta divergência de identidade e interrompe a inicialização
- **Autenticação local ainda deverá ser implementada** antes de afirmar que o proprietário está autenticado
- **Autorização completa por identidade e políticas explícitas ainda deverá ser implementada**
- O **frontend não deve enviar credenciais ou IDs de autorização**
- **authority_override** é um atributo, não uma permissão automática
- O **proprietário não é funcionário** — são identidades distintas
- Este documento contém princípios e requisitos futuros; itens não implementados estão identificados como modelo-alvo