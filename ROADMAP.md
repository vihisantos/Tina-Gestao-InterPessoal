# ROADMAP — TINA Platform

**Versão:** 1.0.0 (2026-11-17)  
**Status:** Planejamento  

---

## Fase 0: Fundação Arquitetural ✅

**Duração prevista:** 3 dias  
**Status:** Concluída  

### Entregas

- [x] Estrutura de diretórios criada
- [x] Stack tecnológica definida e documentada
- [x] Arquitetura modular definida
- [x] Modelos de domínio e entidades especificados
- [x] Documentação de decisões técnicas (TECHNOLOGY_DECISIONS.md)
- [x] Documento de arquitetura (ARCHITECTURE.md)

---

## Fase 1: Aplicativo Desktop Funcional

**Duração prevista:** 2-3 semanas  
**Status:** Em andamento (bootstrap)  

### Objetivo

Criar um aplicativo desktop Tauri mínimo viável que:
- Compile e abra sem conexão à internet
- Persista dados localmente em SQLite
- Funcione sem serviços remotos

### Entregas

| Item | Descrição | Critério de Aceitação |
|------|-------------|----------------------|
| 1.1 | Configuração do projeto Rust | `cargo build` completa com sucesso |
| 1.2 | Tauri setup com SvelteKit | UI abre e mostra contador de exemplo |
| 1.3 | Database layer (SQLite) | Schema inicial aplicado, migrações funcionais |
| 1.4 | Sistema de configuração | Arquivo `tina.toml` criado com defaults |
| 1.5 | Sistema de credenciais | Keyring integrado, fallback criptografado |
| 1.6 | Sistema de logging | Logs escritos para arquivo em debug mode |
| 1.7 | Testes unitários | 70%+ cobertura no módulo infra |

### Tarefas Detalhadas

```
1.1.1 Criar Cargo.toml com dependências mínimas
1.1.2 Configurar target diretório
1.1.3 Verificar build em Windows

1.2.1 Criar projeto Tauri v2
1.2.2 Configurar SvelteKit como frontend
1.2.3 Estabelecer comunicação Rust↔JS
1.2.4 UI inicial (Tela de Boas-vindas/Configuração)

1.3.1 Definir schema inicial SQLite
1.3.2 Implementar migrações (sqlx migrate)
1.3.3 Criar repositórios de inicialização
1.3.4 Testar persistência entre reinicializações

1.4.1 Definir estrutura de tina.toml
1.4.2 Implementar parser de configuração
1.4.3 Validar configuração obrigatória
1.4.4 Documentar formatos

1.5.1 Implementar CredentialStore
1.5.2 Testar Windows Credential Manager
1.5.3 Implementar fallback criptografado
1.5.4 Documentar recuperação de credenciais

1.6.1 Configurar tracing-subscriber
1.6.2 Definir níveis de log
1.6.3 Escrever logs para %APPDATA%\tina\logs
1.6.4 Implementar log rotation

1.7.1 Criar testes de unidade para infra
1.7.2 Testes de integração para database
1.7.3 Testes de configuração
1.7.4 Verificar cobertura com cargo tarpaulin
```

---

## Fase 2: Tina Core & Planificação

**Duração prevista:** 3-4 semanas  
**Status:** Pendente  

### Objetivo

Implementar o núcleo da Tina com capacidades de planejamento e coordenação.

### Entregas

| Item | Descrição | Critério de Aceitação |
|------|-------------|----------------------|
| 2.1 | Tina Core (coordinator) | Recebe objetivo, delega a planificação |
| 2.2 | Planner Engine | Decompõe objetivos em tarefas |
| 2.3 | Context Manager | Seleciona contexto relevante |
| 2.4 | Organização Manager | Cria empresas, departamentos |
| 2.5 | Hierarchy Engine | Gerencia cargos e hierarquia |

### Tarefas Detalhadas

```
2.1.1 Criar entidade Installation
2.1.2 Criar entidade Owner (proprietário)
2.1.3 Implementar Coordinator struct
2.1.4 Integrar scheduler de tarefas
2.1.5 Testar fluxo básico

2.2.1 Implementar ObjectiveInterpreter
2.2.2 Implementar TaskDecomposer
2.2.3 Implementar DependencyGraph
2.2.4 Criar templates de planificação
2.2.5 Testar decomposição recursiva

2.3.1 Implementar ContextSelector
2.3.2 Implementar ContextDistributor
2.3.3 Implementar ContextConsolidator
2.3.4 Testar filtragem por escopo
2.3.5 Integrar com memória

2.4.1 Criar tabela companies
2.4.2 Criar tabela departments
2.4.3 Implementar CompanyService
2.4.4 Implementar DepartmentService
2.4.5 Testar isolamento entre empresas

2.5.1 Criar tabela positions
2.5.2 Criar tabela employees
2.5.3 Implementar HierarchyBuilder
2.5.4 Implementar AuthorityCalculator
2.5.5 Testar cadeias de comando
```

---

## Fase 3: Agente & Model Router

**Duração prevista:** 4-5 semanas  
**Status:** Pendente  

### Objetivo

Implementar sistema de agentes com integração NVIDIA.

### Entregas

| Item | Descrição | Critério de Aceitação |
|------|-------------|----------------------|
| 3.1 | Agent Manager | Cria, lista, gerencia funcionários |
| 3.2 | NVIDIA NIM Client | Conexão com API funcional |
| 3.3 | Model Router | Seleção dinâmica de modelos |
| 3.4 | Memory Service | Armazenamento de memórias |
| 3.5 | MCP Integration | Conexão com servidores MCP |

### Tarefas Detalhadas

```
3.1.1 Implementar Employee entity
3.1.2 Implementar Position entity
3.1.3 Criar EmployeeService
3.1.4 Criar PositionService
3.1.5 Testar hierarquia de funcionários

3.2.1 Criar NvidiaNimClient struct
3.2.2 Implementar chat_completion
3.2.3 Implementar streaming endpoints
3.2.4 Testar com API key válida
3.2.5 Tratar erros de autenticação

3.3.1 Implementar ModelRegistry
3.3.2 Implementar ModelSelector
3.3.3 Implementar cost_estimator
3.3.4 Testar seleção por cargo
3.3.5 Testar override de modelo

3.4.1 Implementar Memory entity
3.4.2 Implementar MemoryService
3.4.3 Implementar VectorStore (manual)
3.4.4 Implementar EmbeddingGenerator
3.4.5 Testar persistência de memória

3.5.1 Configurar rmcp client
3.5.2 Implementar ToolRegistry
3.5.3 Criar adapters MCP comuns
3.5.4 Testar com MCP servers
```

---

## Fase 4: Execução & Permissões

**Duração prevista:** 3-4 semanas  
**Status:** Pendente  

### Objetivo

Implementar sistema de execução com controle de permissões robusto.

### Entregas

| Item | Descrição | Critério de Aceitação |
|------|-------------|----------------------|
| 4.1 | Execution Engine | Executa ferramentas localmente |
| 4.2 | Permission Engine | Controle de acesso granular |
| 4.3 | Evaluation Engine | Valida resultados |
| 4.4 | Sandbox System | Isolamento de execução |

### Tarefas Detalhadas

```
4.1.1 Implementar ToolRegistry
4.1.2 Criar Tool trait
4.1.3 Implementar FileSystemTool
4.1.4 Implementar ProcessTool
4.1.5 Testar execução segura

4.2.1 Implementar PolicyEngine
4.2.2 Implementar RuleChecker
4.2.3 Implementar PermissionMatrix
4.2.4 Testar hierarquia de permissões
4.2.5 Testar acesso do proprietário

4.3.1 Implementar Validator trait
4.3.2 Implementar QualityScorer
4.3.3 Implementar ComplianceChecker
4.3.4 Criar métricas de avaliação
4.3.5 Testar validação de resultados

4.4.1 Implementar Sandbox (restricted env)
4.4.2 Criar ProcessLimiter
4.4.3 Implementar Timeout system
4.4.4 Testar isolamento de recursos
4.4.5 Testar cancelamento de tarefas
```

---

## Fase 5: Activity System & UI

**Duração prevista:** 4-6 semanas  
**Status:** Pendente  

### Objetivo

Criar sistema de visibilidade completo e interface do proprietário.

### Entregas

| Item | Descrição | Critério de Aceitação |
|------|-------------|----------------------|
| 5.1 | Event Bus | Comunicação inter-componentes |
| 5.2 | Activity System | Tracking de eventos |
| 5.3 | WebSocket API | Comunicação em tempo real |
| 5.4 | Tauri Frontend | UI completa funcional |
| 5.5 | Comando Central | Painel de controle |

### Tarefas Detalhadas

```
5.1.1 Implementar EventBus (broadcast channels)
5.1.2 Implementar EventStore
5.1.3 Criar EventType enum
5.1.4 Testar publicação/assinatura

5.2.1 Implementar EventTracker
5.2.2 Implementar ActivityLog
5.2.3 Implementar TimelineView
5.2.4 Testar persistência de eventos

5.3.1 Configurar WebSocket server
5.3.2 Implementar event streaming
5.3.3 Criar endpoint de subscribes
5.3.4 Testar conexão múltipla clientes

5.4.1 Criar tela inicial (Welcome)
5.4.2 Criar tela de configuração
5.4.3 Implementar navegação entre telas
5.4.4 Adicionar dark/light theme
5.4.5 Testar responsividade desktop

5.5.1 Criar Dashboard do proprietário
5.5.2 Implementar Organograma visual
5.5.3 Criar lista de funcionários
5.5.4 Implementar detalhe de delegação
```

---

## Fase 6: Testes & Refinamento

**Duração prevista:** 2-3 semanas  
**Status:** Pendente  

### Objetivo

Garantir qualidade, estabilidade e preparar para release.

### Entregas

| Item | Descrição | Critério de Aceitação |
|------|-------------|----------------------|
| 6.1 | Testes de integração | 80%+ das rotas críticas |
| 6.2 | Testes de carga | Simulação de 100 agentes |
| 6.3 | Documentação | Documentação completa de API |
| 6.4 | Release Candidate | Build estável disponível |

### Tarefas Detalhadas

```
6.1.1 Criar testes de API endpoints
6.1.2 Testar fluxo completo Tina→Agente
6.1.3 Testar delegação/delegado
6.1.4 Testar memória persistente
6.1.5 Testar reconexão NVIDIA API

6.2.1 Simular múltiplas empresas
6.2.2 Simular múltiplos departamentos
6.2.3 Testar performance com 1000 delegations
6.2.4 Testar tempo de resposta
6.2.5 Testar uso de memória

6.3.1 Gerar OpenAPI spec
6.3.2 Documentar arquitetura
6.3.3 Documentar API Rust
6.3.4 Criar guia de instalação
6.3.5 Criar guia de troubleshooting

6.4.1 Verificar build release
6.4.2 Testar instalador
6.4.3 Verificar assinatura do binário
6.4.4 Criar changelog
6.4.5 Publicar release candidate
```

---

## Timeline Consolidado

```
Mês 1: Fase 0 + Fase 1 (Fundação + Desktop inicial)
Mês 2: Fase 2 + Fase 3 (Core + Agentes)
Mês 3: Fase 4 + Fase 5 (Execução + UI)
Mês 4: Fase 6 (Testes + Release)
```

---

## Dependências Críticas

1. **API Key NVIDIA** - Necessária para testes de integração (Fase 3)
2. **Certificados de desenvolvedor** - Para assinatura de binário (Fase 6)
3. **Modelos de embedding locais** - Para memória vetorial (Pós-Fase 1)

---

## Riscos e Mitigações

| Risco | Probabilidade | Impacto | Mitigation |
|-------|--------------|---------|------------|
| Cursor de aprendizado Rust | Alto | Médio | Pair programming, code reviews |
| API Key exposta | Baixo | Crítico | Kering Windows, logs sanitizados |
| SQLite em escala | Médio | Médio | Monitoring, planejamento para futuro upgrade |
| Tauri 2 API instável | Médio | Médio | Seguir roadmap oficial, PRs de feedback |

---

## Critérios de Aceitação por Fase

### Fase 1: Desktop Inicial
- [ ] Aplicativo compila e abre em Windows 10+
- [ ] Banco de dados SQLite criado automaticamente
- [ ] Configuração salva e carrega corretamente
- [ ] Credenciais armazenadas com segurança
- [ ] Logs escritos para diretório apropriado
- [ ] Testes de unidade passam

### Fase 2: Core & Planificação
- [ ] Proprietário pode criar organização
- [ ] Departamentos podem ser criados
- [ ] Cargos podem ser definidos
- [ ] Hierarquia é respeitada em consultas
- [ ] Contexto é selecionado corretamente

### Fase 3: Agentes & NVIDIA
- [ ] Conexão NVIDIA API funcional
- [ ] Modelos podem ser selecionados
- [ ] Streaming funciona corretamente
- [ ] Memórias são persistidas
- [ ] MCP servers podem ser conectados

### Fase 4: Execução & Permissões
- [ ] Ferramentas executam com permissão correta
- [ ] Proprietário tem acesso total
- [ ] Resultados são validados
- [ ] Sandbox funciona conforme esperado
- [ ] Ações são auditadas

### Fase 5: UI & Comando
- [ ] Comando Central é funcional
- [ ] WebSocket delivera eventos
- [ ] UI é responsiva
- [ ] Histórico de atividades visível
- [ ] Integração frontend-backend sólida

### Fase 6: Qualidade
- [ ] Cobertura de testes > 75%
- [ ] Build release estável
- [ ] Documentação completa
- [ ] Performance aceitável
- [ ] Zero vulnerses críticas

---

*Este roadmap será revisado e atualizado conforme feedback do proprietário e progresso da equipe.*