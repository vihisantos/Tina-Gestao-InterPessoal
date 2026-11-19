<p align="center">
  <img src="src-frontend/static/tina.png" alt="Tina, a mascote rosa do aplicativo" width="180" />
</p>

<h1 align="center">Tina — Agência de Gestão</h1>

<p align="center">
  <em>Inteligência pessoal e operacional — fundo branco, alma rosa.</em>
</p>

<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Ícone do aplicativo Tina" width="88" />
</p>

**Status:** Fase 1 — Desktop Inicial (bootstrap em andamento)

---

## O que é a Tina

A Tina é uma plataforma de inteligência pessoal e operacional: uma equipe de agentes de IA com papel, responsabilidade, hierarquia e memória própria, trabalhando sob a autoridade máxima do proprietário.

Ela foi concebida para pequenas e médias empresas que querem operação séria, com disciplina, sem abrir mão da soberania sobre os próprios dados.

### A Tina tem rosto

A Tina não é um nome genérico: é uma personagem. Uma criatura rosa, de corpo
arredondado e macio, sem boca, dois olhos ovais pretos e bracinhos erguidos — sempre
flutuando, como quem acabou de chegar para resolver o dia. Essa personagem **é** a
identidade do produto: aparece no logo do app, dentro das telas como mascote, no
ícone da área de trabalho e no favicon da janela.

Toda a interface nasce dela: fundo branco, rosa em cada detalhe, nada de tema escuro.
Quem alterar UI, ícones ou assets deve seguir o [AGENTS.md](AGENTS.md), que registra a
personagem, os arquivos oficiais e os tokens de cor.

---

## Identidade visual

| | |
|---|---|
| <img src="src-frontend/static/tina.png" alt="Tina" width="120" /> | **Personagem:** mascote rosa, corpo arredondado, olhos ovais pretos, sem boca, flutuando. Arte em `src-frontend/static/tina.png`. |
| <img src="src-tauri/icons/icon.png" alt="Ícone" width="120" /> | **Ícone:** a mesma personagem em tile de cantos arredondados, fundo `#fff7fa` → `#ffe2ed`, borda `#ffc7db`. Gerado em `src-tauri/icons/`. |

| Cor | Hex | Onde |
|---|---|---|
| Rosa Tina | `#ee6495` | Cor principal, botões, destaques |
| Rosa forte | `#d94f80` | Hover, gradiente do botão |
| Rosa profundo | `#c9266b` | Texto de ênfase |
| Rosa suave | `#f7a8c4` | Detalhes e bordas de destaque |
| Branco | `#ffffff` | Fundo de todas as telas |
| Rosa de fundo | `#fff7fa` | Chips, cards, realces |

Os tokens ficam em [`src-frontend/src/theme.css`](src-frontend/src/theme.css) e são
consumidos como `var(--tina-*)`. A UI reutiliza
[`Logo`](src-frontend/src/lib/components/Logo.svelte),
[`Card`](src-frontend/src/lib/components/Card.svelte),
[`Badge`](src-frontend/src/lib/components/Badge.svelte) e
[`Button`](src-frontend/src/lib/components/Button.svelte).

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

## O que a Tina vai fazer

O objetivo final é uma **agência de gestão interpessoal** rodando na máquina do
proprietário, sem depender de servidor. Nas fases seguintes do
[ROADMAP.md](ROADMAP.md):

1. **Entender o pedido, não só o comando.** A Tina Central recebe um objetivo e o
decompõe em tarefas com plano, contexto e critério de conclusão (Fase 2).
2. **Montar a organização.** Cria empresas, departamentos, cargos e hierarquia, e
   contrata funcionários de IA com papel, chefe e responsabilidade definidos (Fases 2–3).
3. **Conversar com modelos NVIDIA.** Roteia cada tarefa para o modelo adequado via
   NVIDIA NIM, com streaming e tratamento de falha de credencial (Fase 3).
4. **Lembrar.** Mantém memória pessoal, profissional e organizacional, com busca
   vetorial local no SQLite — o contexto certo no momento certo (Fase 3).
5. **Executar com limites.** Ferramentas de arquivo e processo em sandbox, com
   permissões granulares e avaliação do resultado antes de valer (Fase 4).
6. **Prestar contas.** Cada evento fica registrado e visível no Comando Central:
   quem pediu, quem executou, o que mudou (Fase 5).

O princípio que atravessa tudo: **a autoridade é do proprietário**. A Tina propõe,
executa o que foi autorizado e deixa rastro — nunca age fora do escopo concedido.

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

- **[AGENTS.md](AGENTS.md)** - Identidade da personagem e regras de UI para agentes de IA
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