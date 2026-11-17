# ARCHITECTURE — TINA Platform

**Versão:** 1.0.1 (2026-11-17) - Revisão pós-missão 001  
**Status:** Documento de Fundação Arquitetural  

---

## 1. Visão Geral

A Tina é uma plataforma de inteligência pessoal e operacional que atua como **inteligência central** e **chefe das assistentes**. Ela é projetada para ser uma arquitetura modular, expansível e independente, capaz de acompanhar toda a vida do usuário e administrar organizações, empresas, departamentos, equipes e agentes especializados.

### 1.1 Princípios Arquiteturais

1. **Modularidade:** Cada componente tem responsabilidade única e bem definida.
2. **Segurança:** Permissões granulares, isolamento entre contextos e auditoria.
3. **Local-first:** Operar offline, dados locais, sem dependência de infraestrutura externa.
4. **NVIDIA como provedor de IA:** Modelos Nemotron via NIM API (híbrido: local ou cloud).
5. **Abstração de provedores:** Camada de router que permite troca de provedores sem impacto no cargo.
6. **Hierarquia flexível:** Agentes com cadeias de comando configuráveis.
7. **Memória persistente:** Memória pessoal, profissional e organizacional separadas.

---

## 2. Stack Tecnológica Atualizada

### 2.1 Linguagem Principal

**Rust** — Escolhido por:
- Zero-cost abstractions para performance sem overhead
- Memory safety sem GC, ideal para dados sensíveis
- Single binary, sem runtime
- Excelente para Windows

### 2.2 Backend (INTERNAL ONLY)

**Apenas para uso interno do Tauri.**

```
┌────────────────────────────────────────────┐
│              Tauri 2 Runtime               │
├────────────────────────────────────────────┤
│  Rust Core (Tina Logic)                    │
│  ├─ Config/Init                           │
│  ├─ Database (SQLite via sqlx)            │
│  ├─ Business Logic (all modules)          │
│  └─ State Management (Arc<Mutex>)         │
├────────────────────────────────────────────┤
│  Tauri API Bridge                          │
│  ├─ invoke() → Rust functions             │
│  └─ event emitter for async updates       │
└───────────────────────┬───────────────────┘
                        │
┌───────────────────────▼───────────────────┐
│  SvelteKit/React Frontend                 │
│  ├─ UI Components                         │
│  ├─ State Management (Redux/SvelteKit)    │
│  └─ Tauri Bindings                        │
└────────────────────────────────────────────┘
```

### 2.3 Frontend

**Tauri 2 + SvelteKit**

| Critério | Tauri | Electron |
|----------|-------|----------|
| Tamanho do binário | ✅ ~15-25MB | ~100-200MB |
| Performance | ✅ WebView otimizado | Chromium completo |
| Segurança | ✅ Contexto limitado | Node.js integration (perigo) |
| Local-first | ✅ Arquivos locais | Arquivos locais |

### 2.4 Banco de Dados

**SQLite 3** com:
- Migrations versionadas via sqlx
- Transações ACID para operações críticas
- JSON colunar via sqlite-json extension
- WAL mode para performance

### 2.5 Memória e Busca Semântica

**Arquitetura híbrida (implementada em Rust):**

```
┌─────────────────────────────────────────────────────────────┐
│                    Memory System                             │
├────────────────────────┬────────────────────────────────────┤
│   Vector Search        │   Full-Text Search                 │
│   (Custom Rust impl)   │   (SQLite FTS)                     │
│   - Cosine similarity  │   - to_tsvector()                  │
│   - HNSW-like algo     │   - BM25 ranking                   │
├────────────────────────┴────────────────────────────────────┤
│                    Hybrid Fusion (RRF)                       │
│  Reciprocal Rank Fusion para combinar resultados             │
└─────────────────────────────────────────────────────────────┘
```

### 2.6 Orquestração de Agentes

**Rust-native agent loop** com:

- **Pydantic AI**-inspirado: arquitetura de agent typesafe
- **OpenAI Agents SDK** patterns: handoffs, subagents, guardrails
- **MCP Integration**: Somente para ferramentas locais autorizadas

### 2.7 Filas e Processamento Assíncrono

**Broadcast channels** para eventos internos e **SQLite WAL** para persistência.

```
┌──────────────┐     ┌──────────────┐
│  Tina Core   │────▶│  Planner     │────▶│
└──────────────┘     └──────┬───────┘
                            ▼
                     ┌──────────────┐
                     │  Activity    │
                     │  System      │
                     └──────┬───────┘
                            ▼
                     ┌──────────────┐
                     │  SQLite      │
                     │  (WAL mode)  │
                     └──────────────┘
```

### 2.8 Comunicação Interna

```
┌─────────────────┐  ┌─────────────────┐
│  Frontend UI    │  │  Background     │
│  (SvelteKit)    │  │  Tasks          │
└───────┬─────────┘  └────────┬────────┘
        │                     │
        ▼                     ▼
┌──────────────────────────────────────────┐
│           Tauri Bridge/API               │
│  ├─ invoke() → Rust functions            │
│  └─ emit() ← Event callbacks             │
└──────────────────┬───────────────────────┘
                   │
┌──────────────────▼───────────────────────┐
│           Tina Rust Core                   │
│  ┌─────────────┐  ┌─────────────┐         │
│  │  HTTP API   │  │  WebSocket  │         │
│  │  (disabled) │  │  (disabled) │         │
│  └─────────────┘  └─────────────┘         │
└──────────────────────────────────────────┘
```

### 2.9 Infraestrutura de Modelos

**Camada de abstração do Model Router:**

```
┌────────────────────────────────────────────┐
│              Model Router                  │
├────────────────────────────────────────────┤
│  Provider Interface (trait)              │
│  ├─ NVIDIA NIM API (primary)              │
│  ├─ Local NIM Container (future)          │
│  └─ OpenAI-Compatible (future, optional)  │
├────────────────────────────────────────────┤
│  Model Registry                            │
│  - Per role/position mappings              │
│  - Dynamic model selection                 │
│  - Cost/quality trade-offs                 │
└────────────────────────────────────────────┘
```

### 2.10 Testes e Observabilidade

- **Testing:** cargo test + tarpaulin para cobertura
- **Tracing:** OpenTelemetry via tracing crate (LOCAL ONLY)
- **Logs:** Escritos para arquivo em `%APPDATA%\tina\logs`
- **Metrics:** Métricas em memória, exposta via Tauri IPC

---

## 3. Estrutura de Diretórios

```
Tina-Gestao-InterPessoal/
├─ src/
│  ├─ lib.rs                           # Biblioteca do núcleo (sem entry point)
│  ├─ config.rs                        # Configuração (TOML)
│  ├─ error.rs                         # Erros customizados
│  │
│  ├─ core/                            # Tina Core
│  │  ├─ mod.rs
│  │  ├─ tina.rs                       # Entidade Tina Central
│  │  └─ scheduler.rs                  # Agendador de tarefas
│  │
│  ├─ planner/                         # Planner Engine
│  ├─ context/                         # Context Manager
│  ├─ agent/                            │ # Agent Manager
│  ├─ route/                            │ # Model Router
│  ├─ memory/                           │ # Memory System
│  ├─ auth/                             │ # Tina Auth
│  ├─ exec/                             │ # Execution Engine
│  ├─ eval/                             │ # Evaluation Engine
│  ├─ activity/                         │ # Activity System
│  ├─ infra/                            │ # Infrastructure
│  ├─ adapter/                          │ # Adaptadores (API only)
│  └─ mcp/                               │ # MCP Integration (tools only)
│
├─ migration/                          # 001_initial.sql (SQLite idempotente)
├─ tests/
├─ package.json                        # Workspace npm + CLI Tauri
├─ src-tauri/                          # Tauri 2: main.rs, build.rs, tauri.conf.json
├─ Cargo.toml                          # Workspace + núcleo Rust
└─ Cargo.lock
```

---

## 4. Diagrama de Componentes Atualizado

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Tina Platform Architecture                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────┐  ┌─────────────────┐                                      │
│  │     User        │  │   Owner UI      │                                      │
│  │  (Human)        │  │  (Tauri/Svelte) │                                      │
│  └────────┬────────┘  └────────┬────────┘                                      │
│           │                      │                                            │
│           │         ┌────────────▼────────────┐                               │
│           │         │      Tauri Bridge        │                               │
│           │         │  (invoke / emit events)  │                               │
│           │         └────────────┬────────────┘                               │
│           │                      │                                            │
│           │         ┌────────────▼────────────┐                               │
│           │         │    Tina Core Engine     │                               │
│           │         │   (coordinator.rs)      │                               │
│           │         └────────────┬────────────┘                               │
│           │                      │                                            │
│  ┌────────▼───────────────────────────────────────────────▼─────────────┐  │
│  │                    Planner Engine                              │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                     Context Manager                               │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                    Agent Manager                                    │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                    Model Router                                     │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │  │
│  │  │  Selector   │  │  Provider   │  │  NVIDIA NIM │                  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                  │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌─────────────────────────────────┐  ┌────────────────────────────────┐ │
│  │     Memory System               │  │    Permission Engine           │ │
│  └─────────────────────────────────┘  └────────────────────────────────┘ │
│                                                                              │
│  ┌─────────────────────────────────┐  ┌────────────────────────────────┐ │
│  │  Evaluation Engine              │  │   Execution Engine           │ │
│  └─────────────────────────────────┘  └────────────────────────────────┘ │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                   Activity System                                     │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                    Data Stores                                      │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │  │
│  │  │  SQLite     │  │  Files      │  │  Cache      │                  │  │
│  │  │  (main)     │  │             │  │ (in-memory) │                  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                  │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │  External Communication                                           │  │
│  │  ┌─────────────┐  ┌─────────────┐                                 │  │
│  │  │  NVIDIA NIM │  │  MCP Tools  │  (only authorized/local)          │  │
│  │  │    API      │  │   (local)   │                                 │  │
│  │  └─────────────┘  └─────────────┘                                 │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Fluxo de Comunicação Atualizado

```
Human → Tina Core → Planner Engine → Context Manager
    ↓
Model Router ← Memory System ← Event Bus
    ↓
NVIDIA NIM API (EXTERNAL, AI INFERENCE ONLY)
    ↑
Activity System → Human (via Tauri Bridge)
```

---

## 5. Segurança das Credenciais

### 5.1 Armazenamento da NVIDIA API Key

**Mecanismo:** Windows Credential Manager (via `keyring` crate)

```rust
use keyring::Entry;

pub struct CredentialStore {
    entry: Entry,  // Windows Credential Manager entry
}

impl CredentialStore {
    pub fn store(&self, secret: &str) -> Result<()> {
        self.entry.set_secret(secret)?;  // Armazena no Locker do Windows
        Ok(())
    }

    pub fn retrieve(&self) -> Result<String> {
        self.entry.get_secret()  // Recupera do Locker
    }
}
```

**Fluxo de armazenamento:**
1. Usuário fornece API Key via UI Tauri
2. Rust salva no Windows Credential Manager (criptografado por Windows)
3. Em reinicialização, aplicação recupera automaticamente
4. Nenhuma chave é gravada em disco ou no repositório

### 5.2 Uso do Argon2id

**Finalidade:** Hasheamento de senhas para:
- Autenticação do proprietário na interface
- Transição de sessão (quando implementada)
- Proteção de senhas no banco de dados

**Implementação:**
```rust
use argon2::{Argon2, PasswordHasher, PasswordVerifier};

pub fn hash_password(password: &str) -> Result<String> {
    let salt = rand::random::<[u8; 16]>();
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}
```

### 5.3 Outras Informações Sensíveis

| Dado | Proteção |
|------|----------|
| Senhas | Argon2id hash no banco |
| Chave API | Windows Credential Manager |
| Memórias sensíveis | Criptografia AES-256-GCM sob demanda |
| Tokens de sessão | JWT com assinatura HS256 |
| Arquivos do usuário | Nenhum acesso não autorizado via sandbox |

---

## 6. Organização Dinâmica

### 6.1 Estrutura de Dados

```sql
-- Empresas são criadas dinamicamente
INSERT INTO companies (id, installation_id, name, status)
VALUES (?, ?, ?, 'active');

-- Departamentos dentro de empresas
INSERT INTO departments (id, company_id, name, code)
VALUES (?, ?, ?, ?);

-- Cargos com níveis de autoridade configuráveis  
INSERT INTO positions (id, department_id, name, level, authority_level)
VALUES (?, ?, ?, ?, ?);

-- Funcionários IA com modelos configuráveis
INSERT INTO employees (id, position_id, name, model_config, ...)
VALUES (?, ?, ?, ?, ...);
```

### 6.2 Fluxo de Criação Dinâmica

```
Owner → Tina Core → Organization Manager
    ↓
[CREATE Company(name: "Empresa A")]
    ↓
[CREATE Department(code: "FIN", name: "Financeiro")]
    ↓
[CREATE Position(name: "Analista", level: 2, authority: 5)]
    ↓
[CREATE Employee(name: "Fin-IA-01", position_id: ..., model: "...")]
    ↓
Result: Nova organização com estrutura completa
```

---

## 7. Autoridade Máxima do Proprietário

### 7.1 Verificação de Autoridade

```rust
pub fn owner_can_access(&self, employee: &Employee) -> bool {
    employee.id == self.owner_id  // ID do proprietário
}

pub fn owner_command(&self, target: &str, command: &str) -> Result<()> {
    // Acesso imediato sem intermediação
    self.agent_manager.interrupt(target)?;
    self.agent_manager.send_command(target, command)?;
    Ok(())
}
```

### 7.2 Recursos do Proprietário

| Recurso | Funcionalidade |
|---------|---------------|
| Chat direto | `conversation.start(employee_id, owner_initiated=true)` |
| Consulta atividades | `events.list_by_employee(employee_id)` |
| Emissão ordens | `delegations.create(delegante_id=owner, ...)` |
| Pausa interrupção | `tasks.pause(id)`, `tasks.cancel(id)` |

---

## 8. Isolamento entre Organizações

### 8.1 Nível de Isolamento

```
┌────────────────────────────────────────────┐
│           Installation (owner)              │
├─────────────────────────────────────────────┤
│  Company A          │  Company B           │
│  ┌─────────────┐    │  ┌─────────────┐     │
│  │Dept 1       │    │  │Dept 1       │     │
│  │┌──────────┐ │    │  │┌──────────┐ │     │
│  ││Emp 1    │ │    │  ││Emp 1    │ │     │
│  ││Emp 2    │ │    │  ││Emp 2    │ │     │
│  │└──────────┘ │    │  │└──────────┘ │     │
│  └─────────────┘    │  └─────────────┘     │
└───────────────────────┬────────────────────┘
                        │
               CROSS-COMPANY ACCESS
               [REQUEST → Owner Approval]
```

### 8.2 Implementação

```sql
-- Todas as queries usam company_id como filtro
SELECT * FROM memories 
WHERE company_id = ? AND scope = ?;

SELECT * FROM employees e
JOIN positions p ON e.position_id = p.id
JOIN departments d ON p.department_id = d.id
WHERE d.company_id = ?;
```

---

## 9. Funcionamento Offline

### 9.1 Componentes que funcionam offline

| Componente | Status Offline |
|------------|---------------|
| UI (Tauri/Svelte) | ✅ Totalmente funcional |
| Banco de dados | ✅ SQLite local |
| Memória | ✅ Busca local (similaridade + FTS) |
| Hierarquia | ✅ Totalmente funcional |
| Ferramentas locais | ✅ Sandbox local |
| Delegação | ✅ Totalmente funcional |
| Eventos/tracking | ✅ Totalmente funcional |

### 9.2 Dependência de rede

| Serviço | Necessário Online? |
|---------|-------------------|
| Interface | ❌ Não |
| Banco de dados | ❌ Não |
| Autenticação | ❌ Não (Keyring local) |
| Inteligência AI | ✅ Sim (NVIDIA NIM API) |
| MCP servidores | ❌ Não (ferramentas locais apenas) |
| Telemetria | ❌ Não (local only) |

---

## 10. Critérios de Aceitação - Fase 1 Atualizados

### 10.1 Desktop Inicial

| Critério | Descrição | Evidence |
|----------|-------------|-------------|
| F1-1 | Aplicativo Tauri abre | `tauri dev` funciona |
| F1-2 | SQLite criado automaticamente | Arquivo `tina.db` criado |
| F1-3 | Migrações aplicadas | Schema inicial aplicado |
| F1-4 | Tauri API responde | `invoke()` funciona |
| F1-5 | UI carrega | Tela inicial visível |
| F1-6 | Configuração persistida | `tina.toml` criado |
| F1-7 | Credenciais seguras | Keyring integrado |

### 10.2 Organização Dinâmica

| Critério | Descrição | Evidence |
|----------|-------------|-------------|
| F1-8 | Criar empresa | `CREATE Company` funciona |
| F1-9 | Criar departamento | `CREATE Department` funciona |
| F1-10 | Criar cargo | `CREATE Position` funciona |
| F1-11 | Criar funcionário | `CREATE Employee` funciona |
| F1-12 | Modelos configuráveis | `UPDATE Employee SET model_config` funciona |

### 10.3 Segurança

| Critério | Descrição | Evidence |
|----------|-------------|-------------|
| F1-13 | Keyring funcional | Chave recuperada com sucesso |
| F1-14 | Argon2id aplicado | Hash verificado |
| F1-15 | Isolamento empresa | `SELECT` com `WHERE company_id` |
| F1-16 | Owner tem acesso total | `WHERE employee_id = owner` |

### 10.4 Offline

| Critério | Descrição | Evidence |
|----------|-------------|-------------|
| F1-17 | UI funciona sem internet | Desktop abre offline |
| F1-18 | Banco funciona offline | Operações SQL offline |
| F1-19 | Memória funciona offline | Busca local funciona |

---

## 11. Próximos Passos

1. **Revisão do proprietário** sobre esta arquitetura revisada
2. **Aprovação** dos critérios de aceitação Fase 1
3. **Início da implementação** da Fase 1 (Tauri + SQLite + Config)

---

*Documento revisado com base nas correções solicitadas pelo proprietário. A arquitetura mantém-se modular e expansível, com foco em local-first operation e segurança robusta.*