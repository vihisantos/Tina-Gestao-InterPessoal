# ACCEPTANCE_CRITERIA.md — Critérios de Aceitação

**Versão:** 1.0.0 (2026-11-17)  
**Status:** Documento de aceitação para Fase 0  

---

## 1. Critérios Gerais

### 1.1 Código

| Critério | Descrição | Verificação |
|----------|-------------|-------------|
| GC-1 | Código compila sem warnings | `cargo build` limpo |
| GC-2 | Código passa no clippy | `cargo clippy -- -D warnings` |
| GC-3 | Formatação conforme rustfmt | `cargo fmt -- --check` |
| GC-4 | Cobertura de testes ≥ 70% no core | `cargo tarpaulin` |
| GC-5 | Nenhuma dependência duplicada | `cargo tree` sem conflitos |

### 1.2 Documentação

| Critério | Descrição | Verificação |
|----------|-------------|-------------|
| GD-1 | README.md atualizado | Existe e completo |
| GD-2 | ARCHITECTURE.md completo | Documenta todos os módulos |
| GD-3 | TECHNOLOGY_DECISIONS.md | Justifica todas as escolhas |
| GD-4 | ROADMAP.md | Fases bem definidas |
| GD-5 | Comentários de código (doc strings) | `#` para módulos e funções |

### 1.3 Segurança

| Critério | Descrição | Verificação |
|----------|-------------|-------------|
| GS-1 | Nenhuma chave API no código | `grep -r API_KEY .` vazio |
| GS-2 | Nenhum secret no repositório | `git-secrets` ou equivalente |
| GS-3 | Hashes de senha são Argon2id | Verificado em auth.rs |
| GS-4 | Dados sensíveis criptografados | AES-256-GCM implementado |
| GS-5 | Permissões validadas | Perms checados em execução |

---

## 2. Critérios por Fase

### 2.1 Fase 0 - Bootstrap (CONCLUÍDA)

| Critério | Status | Evidence |
|----------|--------|----------|
| F0-1 | Estrutura de diretórios criada | `tree src/` mostra estrutura |
| F0-2 | Cargo.toml válido | `cargo check` passa |
| F0-3 | Documentação de arquitetura | ARCHITECTURE.md criado |
| F0-4 | Decisões tecnológicas documentadas | TECHNOLOGY_DECISIONS.md criado |
| F0-5 | Modelo de dados definido | migration/001_initial/schema.sql criado |
| F0-6 | Roadmaps criados | ROADMAP.md, RISKS.md, ACCEPTANCE_CRITERIA.md |

### 2.2 Fase 1 - Aplicativo Desktop Inicial

| Critério | Descrição | Evidence |
|----------|-------------|-------------|
| F1-1 | Aplicativo compila e abre | `cargo run` inicia sem erros |
| F1-2 | SQLite criado automaticamente | Arquivo tina.db criado |
| F1-3 | Migrações aplicadas | Schema inicial aplicado |
| F1-4 | Comando Tauri responde | `invoke('ping')` retorna `pong` |
| F1-5 | Evento Tauri emitido | `tina://ready` recebido no frontend |
| F1-6 | Configuração persistida | tina.toml criado com defaults |
| F1-7 | Credenciais seguras | Keyring integrado |

**Testes esperados:**
```bash
cargo test --test integration -- --test-threads=1
cargo test --test unit -- --test-threads=4
```

### 2.3 Fase 2 - Tina Core & Planner

| Critério | Descrição | Evidence |
|----------|-------------|-------------|
| F2-1 | Tina Core inicia | Coordinator responsivo |
| F2-2 | Planner decompõe objetivos | Objetivo → tarefas |
| F2-3 | Contexto é selecionado | Context relevante retornado |
| F2-4 | Organização criada | Empresa persistida |
| F2-5 | Departamentos criados | Múltiplos departamentos |
| F2-6 | Cargos configuráveis | Diferentes níveis de autoridade |

### 2.4 Fase 3 - Agentes & NVIDIA

| Critério | Descrição | Evidence |
|----------|-------------|-------------|
| F3-1 | AgentManager cria funcionários | Funionário persistido |
| F3-2 | Model Router seleciona modelos | Model escolhido por cargo |
| F3-3 | NVIDIA API conecta | Health check passa |
| F3-4 | Chat funciona | Mensagem→resposta |
| F3-5 | Memória persiste | Memória entre reinicializações |
| F3-6 | MCP servers conectáveis | Conexão testada |

### 2.5 Fase 4 - Execução & Permissões

| Critério | Descrição | Evidence |
|----------|-------------|-------------|
| F4-1 | Execution Engine roda ferramentas | Ferramenta executada |
| F4-2 | Sandbox isola execução | Limite de recursos apply |
| F4-3 | Permissões respeitadas | Access negado quando apropriado |
| F4-4 | Owner tem acesso total | Owner acessa qualquer coisa |
| F4-5 | Delegação funciona | Tarefa delegada e concluída |
| F4-6 | Validação de resultados | Result evalutado corretamente |

### 2.6 Fase 5 - Activity System & UI

| Critério | Descrição | Evidence |
|----------|-------------|-------------|
| F5-1 | Eventos são registrados | Event log escrito |
| F5-2 | WebSocket transmite eventos | Event recebido em tempo real |
| F5-3 | Tauri UI abre | Janela principal visível |
| F5-4 | Comando Central funcional | Dashboard carregado |
| F5-5 | Organograma visível | Hierarquia mostrada |
| F5-6 | Histórico acessível | Atividades históricas |

### 2.7 Fase 6 - Testes & Qualidade

| Critério | Descrição | Evidence |
|----------|-------------|-------------|
| F6-1 | Cobertura ≥ 80% core | `cargo tarpaulin --out Xml` |
| F6-2 | Testes de carga ≥ 100 agentes | Simulação passada |
| F6-3 | Documentação de API | OpenAPI spec gerado |
| F6-4 | Build release estável | Binário testado |
| F6-5 | Performance < 500ms | Benchmark passado |
| F6-6 | Zero vulnérabilités | `cargo audit` limpo |

---

## 3. Critérios de Integração

### 3.1 Comunicação desktop (Tauri IPC)

| Critério | Descrição |
|----------|-------------|
| I1 | Comandos registrados | `bootstrap` e `ping` disponíveis via `invoke()` |
| I2 | Evento de prontidão | `tina://ready` emitido no setup do Tauri |
| I3 | Sem servidor remoto | Nenhum listener HTTP/WebSocket na aplicação |

> Os critérios de API HTTP e WebSocket do bootstrap inicial foram removidos:
> a arquitetura aprovada mantém HTTP/WebSocket desabilitados e usa comandos e
> eventos Tauri para a comunicação frontend ↔ backend.

### 3.3 MCP

| Critério | Descrição |
|----------|-------------|
| I9 | Protocolo 2026-07-28 | Versionamento correto |
| I10 | Ferramentas descobertas | `list_tools` funciona |
| I11 | Execução de tool_calls | Tool executado e resultado retornado |

### 3.4 NVIDIA NIM

| Critério | Descrição |
|----------|-------------|
| I12 | OpenAI-compatible API | Formato compatível |
| I13 | Streaming funcional | `stream=true` |
| I14 | Timeout configurável | Timeout aplicado |
| I15 | Modelos dinamicamente selecionáveis | Modelo por cargo/tarefa |

---

## 4. Critérios de Não-Funcional

### 4.1 Performance

| Métrica | Valor | Teste |
|---------|-------|-------|
| Tempo de resposta API | < 500ms | Benchmark |
| Tempo de inicialização | < 3s | Startup time |
| Uso de memória (idle) | < 100MB | Task Manager |
| Uso de memória (ativo) | < 500MB | Peak usage |
| Tamanho do binário | < 30MB | `ls -lh tina.exe` |

### 4.2 Disponibilidade

| Métrica | Valor | Teste |
|---------|-------|-------|
| Disponibilidade | 99.9% | MTTR < 1min |
| Recovery time | < 30s | Reiniciar |
| Backup automático | Diário | Verificado |

### 4.3 Escalabilidade

| Métrica | Valor | Teste |
|---------|-------|-------|
| Agentes simultâneos | ≥ 100 | Simulação |
| Delegações ativas | ≥ 1000 | Load test |
| Tarefas concluídas/min | ≥ 10 | Throughput |

---

## 5. Critérios de Aceitação do Proprietário

### 5.1 Experiência do Usuário

| Critério | Descrição |
|----------|-------------|
| AP-1 | O aplicativo abre sem internet | Local-first funcional |
| AP-2 | Dados permanecem locais | Nenhum upload automático |
| AP-3 | Acesso ao proprietário é imediato | Nenhuma autorização intermediária |
| AP-4 | Histórico visível | Tudo auditável |
| AP-5 | Controle total | Pause, interrompa, cancele |

### 5.2 Confiabilidade

| Critério | Descrição |
|----------|-------------|
| AR-1 | Dados não se perdem | Backup/restauração testada |
| AR-2 | Atualizações sem quebra | Migrations reversíveis |
| AR-3 | Erros tratados gentilmente | Mensagens claras |
| AR-4 | Recuperação automática | Após falha |

---

## 6. Como Verificar

### 6.1 Testes Automatizados

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration

# Coverage report
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --out Xml

# Lint
cargo clippy -- -D warnings
cargo fmt -- --check

# Security audit
cargo audit
```

### 6.2 Testes Manuais

```bash
# 1. Build release
cargo build --release

# 2. Run binary
./target/release/tina

# 3. Test health endpoint
curl http://localhost:8080/health

# 4. Test WebSocket
# Use wscat or similar

# 5. Test database
sqlite3 ~/.config/tina/tina.db ".tables"

# 6. Test configuration
cat ~/.config/tina/tina.toml
```

---

## 7. Sinais de Rejeição

Um PR será rejeitado se:

- ❌ Não passar em testes automatizados
- ❌ Tiver warnings no clippy
- ❌ Expor credenciais
- ❌ Quebrar funcionalidade existente
- ❌ Falhar em cobertura mínima
- ❌ Violar clean-room policy
- ❌ Depender de provedores não autorizados

---

## 8. Aprovação

Este documento requer aprovação do proprietário para:

- [ ] Fase 0 - Bootstrap
- [ ] Fase 1 - Desktop Inicial
- [ ] Fase 2 - Tina Core & Planner
- [ ] Fase 3 - Agentes & NVIDIA
- [ ] Fase 4 - Execução & Permissões
- [ ] Fase 5 - Activity System & UI
- [ ] Fase 6 - Testes & Qualidade

---

*Este documento será revisado e atualizado conforme a evolução do projeto.*