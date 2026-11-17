# Tina — Agência de Gestão

**Status:** Fase 0 - Arquitetura e Fundação

---

## O que é a Tina

A Tina é uma plataforma de inteligência pessoal e operacional: uma equipe de agentes de IA com papel, responsabilidade, hierarquia e memória própria, trabalhando sob a autoridade máxima do proprietário.

Ela foi concebida para pequenas e médias empresas que querem operação séria, com disciplina, sem abrir mão da soberania sobre os próprios dados.

---

## 🚀 Como Iniciar o Desenvolvimento

**Pré-requisitos no Windows:** Rust (via `rustup`, toolchain `stable`), Node.js
18+, WebView2 Runtime (presente no Windows 11) e as *Ferramentas de Build do
Visual Studio* com o componente C++ para compilar Rust.

```bash
# Na raiz do repositório, uma única vez:
npm install

# Abrir a Tina em janela desktop com hot reload:
npx tauri dev
```

#### Como funciona:

1. **Tauri 2** é o entry point do backend (`src-tauri/src/main.rs`).
2. **Tauri inicia o SvelteKit** (`npm run dev`, porta 5173) automaticamente.
3. **Frontend ↔ Rust** usam comandos (`invoke`) e eventos (`emit`) do Tauri —
   não há servidor HTTP nem WebSocket.
4. **Alterações no frontend** → hot reload na janela desktop.
5. **Alterações no Rust** → Tauri recompila e reinicia automaticamente.
6. **SQLite e configuração** → `%APPDATA%\tina\tina.db` e `tina.toml`.

> `npx tauri dev` **não gera instalador**. O empacotamento está desativado no
> `tauri.conf.json` (`bundle.active: false`).

---

## Principais Recursos

- **Inteligência central** - Tina Central coordena todos os agentes
- **Hierarquia flexível** - Cargos, chefes, líderes configuráveis
- **Memória persistente** - Pessoal, profissional e organizacional
- **Modelos NVIDIA** - Acesso exclusivo aos modelos Nemotron via NIM API
- **Execução segura** - Sandbox para ferramentas e ações
- **Auditoria completa** - Todos os eventos são registrados

---

## Arquitetura

A plataforma é modular, com os seguintes componentes principais:

| Módulo | Responsabilidade |
|--------|------------------|
| `core/` | Coordenação central, scheduler |
| `planner/` | Planejamento e decomposição de objetivos |
| `context/` | Seleção e distribuição de contexto |
| `agent/` | Gerenciamento de funcionários IA |
| `route/` | Router de modelos NVIDIA |
| `memory/` | Sistema de memória persistente |
| `auth/` | Autenticação e permissões |
| `exec/` | Engine de execução de ferramentas |
| `eval/` | Validação de resultados |
| `activity/` | Sistema de eventos e tracking |
| `infra/` | Infraestrutura (DB, telemetry) |
| `adapter/` | Adaptadores (NVIDIA NIM, MCP) |
| `mcp/` | Integração Model Context Protocol |

---

## Stack Tecnológica

| Camada | Tecnologia | Justificativa |
|--------|------------|---------------|
| Linguagem | Rust 2021 | Segurança, performance, local-first |
| Runtime | Tokio 1.x | Async, I/O-bound workloads |
| Desktop | Tauri 2 | Binário pequeno, hot reload |
| Frontend | SvelteKit 2 | Reatividade, hot module replacement |
| Banco de Dados | SQLite | Local-first, sem dependência externa |
| Vector Search | Custom Rust | Integrado ao SQLite |
| Auth | Keyring (Windows) + Argon2id | Credenciais seguras |
| Modelos | NVIDIA NIM API | Único provedor autorizado |

---

## Requisitos Principais

- Windows 10 ou superior
- Rust 1.70+ (para compilação)
- Node.js 18+ (para frontend)
- Chave API NVIDIA configurada

---

## Estrutura de Diretórios

```
Tina-Gestao-InterPessoal/
├─ src/                  # Núcleo Rust (biblioteca `tina`, sem entry point)
│  ├─ lib.rs            # Estado TinaApp + inicialização
│  ├─ config.rs         # Configuração TOML em %APPDATA%\tina
│  ├─ error.rs          # Tipos de erro
│  ├─ infra/db.rs       # SQLite + migrations (embed em compile time)
│  └─ core/, planner/, context/, agent/, route/, memory/,
│     auth/, eval/, exec/, activity/, adapter/, mcp/   # preservados,
│     declarados nas fases seguintes
├─ src-frontend/         # SvelteKit + adapter-static
│  ├─ src/routes/       # Páginas (+page.svelte, +layout.ts)
│  ├─ svelte.config.js  # Config SvelteKit
│  ├─ vite.config.ts    # Config Vite
│  └─ package.json      # Dependências do frontend
├─ src-tauri/            # Runtime desktop (Tauri 2)
│  ├─ Cargo.toml        # Dependências Tauri
│  ├─ build.rs          # tauri-build
│  ├─ src/main.rs       # Entry point definitivo + comandos/eventos
│  ├─ tauri.conf.json   # Config Tauri 2 (sem instalador)
│  └─ icons/            # Ícones da aplicação
├─ migration/
│  └─ 001_initial.sql   # Schema SQLite idempotente
├─ package.json          # Workspace npm + CLI Tauri
├─ docs/                 # Documentação e governança
└─ Cargo.toml            # Workspace Rust + dependências do núcleo
```

---

## Documentação

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Arquitetura detalhada da plataforma
- **[TECHNOLOGY_DECISIONS.md](TECHNOLOGY_DECISIONS.md)** - Decisões tecnológicas fundamentadas
- **[ROADMAP.md](ROADMAP.md)** - Plano de implementação por fases

---

## Status da Implementação

| Fase | Status |
|------|--------|
| Fase 0 - Bootstrap | ✅ Concluída |
| Fase 1 - Desktop Inicial | 🔄 Em andamento |
| Fase 2 - Core & Planner | ⬜ Pendente |
| Fase 3 - Agentes | ⬜ Pendente |
| Fase 4 - Execução | ⬜ Pendente |
| Fase 5 - UI | ⬜ Pendente |
| Fase 6 - Testes | ⬜ Pendente |

### Bootstrap da Fase 1

Já existe nesta árvore:

- entry point único em `src-tauri/src/main.rs` (Tauri 2);
- comandos `bootstrap` e `ping` e evento `tina://ready`;
- criação e migração automática de `%APPDATA%\tina\tina.db`;
- primeira tela funcional em SvelteKit, com hot reload;
- schema SQLite idempotente em `migration/001_initial.sql`.

Os módulos de domínio (`agent`, `memory`, `planner`, etc.) estão preservados,
mas ainda não são compilados: não há login, Comando Central, funcionários ou
organograma nesta versão.

---

## Licença

MIT - Ver arquivo LICENSING.md para detalhes.

---

## Contribuindo

Leia [CONTRIBUTING.md](CONTRIBUTING.md) para diretrizes de contribuição.

**Nota:** Esta é uma implementação clean-room baseada em requisitos aprovados pelo proprietário. Não copiamos código ou documentação de implementações existentes.