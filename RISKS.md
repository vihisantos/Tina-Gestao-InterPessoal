# RISKS.md — Matriz de Riscos Técnicos e de Segurança

**Versão:** 1.0.0 (2026-11-17)  
**Status:** Documento de risco  

---

## 1. Visão Geral

Esta matriz identifica os principais riscos associados ao desenvolvimento e implantação da Tina Platform, juntamente com as estratégias de mitigação planejadas.

---

## 2. Riscos Técnicos

### 2.1 Curva de aprendizado Rust

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Alto ⚠️ |
| **Impacto** | Médio ⚠️ |
| **Prioridade** | Média |

**Descrição:** A curva de aprendizado de Rust é íngreme, especialmente para desenvolvedores acostumados a Python/JavaScript. Isso pode atrasar o desenvolvimento inicial.

**Mitigação:**
- Treinamento em andamento da equipe
- Código review rigoroso
- Documentação detalhada de padrões
- Uso de ferramentas de linting (clippy)
- Pair programming para funcionalidades complexas

---

### 2.2 Limitações do SQLite

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Médio ⚠️ |
| **Impacto** | Médio ⚠️ |
| **Prioridade** | Média-Alta |

**Descrição:** SQLite não possui suporte nativo a vetores como pgvector, exigindo implementação própria para busca semântica.

**Mitigação:**
- Implementação de VectorStore em Rust
- Limites de escala definidos (até ~10M vetores)
- Monitoramento de performance
- Plano de migração futuro para PostgreSQL se necessário

---

### 2.3 Complexidade de async Rust

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Médio ⚠️ |
| **Impacto** | Alto ⚠️ |
| **Prioridade** | Média-Alta |

**Descrição:** A programação async em Rust pode ser complexa, com potencial para deadlocks e bugs de concorrência.

**Mitigação:**
- Uso de padrões bem estabelecidos (tokio, async-trait)
- Testes de concorrência
- Monitoring de deadlocks
- Limitação de lock duration

---

### 2.4 Integração NVIDIA API

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Alto ⚠️ |
| **Impacto** | Alto ⚠️ |
| **Prioridade** | Alta |

**Descrição:** A funcionalidade depende de uma API externa, com potencial de limites, latência e custo.

**Mitigação:**
- Chave API armazenada de forma segura (Windows Credential Manager)
- Cache de modelos para reduzir chamadas
- Circuit breaker para falhas de API
- Timeout configurável
- Feedback claro ao usuário sobre custos

---

### 2.5 Tamaño do binário

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Médio ⚠️ |
| **Impacto** | Baixo ⚠️ |
| **Prioridade** | Baixa-Média |

**Descrição:** O binário pode crescer além do esperado devido a dependências.

**Mitigação:**
- Build otimizado com `opt-level = "z"`
- LTO habilitado
- Remoção de recursos não usados
- Binary analyzer para identificar bloat

---

## 3. Riscos de Segurança

### 3.1 Exposição de credenciais NVIDIA

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Baixo ⚠️ |
| **Impacto** | Crítico ⚠️ |
| **Prioridade** | Alta |

**Descrição:** Se a chave API NVIDIA for exposta, alguém poderia usar os créditos da conta.

**Mitigação:**
- Armazenamento no Windows Credential Manager
- Logs sanitizados (nunca logar chaves)
- Variáveis de ambiente não exportadas
- Criptografia opcional em disco
- Audit trail de uso da API

---

### 3.2 Acesso não autorizado a agentes

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Baixo ⚠️ |
| **Impacto** | Crítico ⚠️ |
| **Prioridade** | Alta |

**Descrição:** Falha na verificação de permissões poderia permitir acesso indevido a agentes ou dados.

**Mitigação:**
- Sistema de permissões baseado em hierarquia
- RBAC (Role-Based Access Control)
- Verificação em múltiplas camadas
- Logs de acesso para auditoria
- Autenticação obrigatória

---

### 3.3 Execução de comandos maliciosos

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Médio ⚠️ |
| **Impacto** | Crítico ⚠️ |
| **Prioridade** | Alta |

**Descrição:** Agentes maliciosos ou corrompidos poderiam executar comandos perigosos no sistema.

**Mitigação:**
- Sandbox de execução separado
- Whitelist de ferramentas permitidas
- Timeout de execução
- Limite de recursos (CPU, memória)
- Validação de inputs antes da execução
- Audit trail obrigatório

---

### 3.4 Cross-company data leak

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Baixo ⚠️ |
| **Impacto** | Crítico ⚠️ |
| **Prioridade** | Alta |

**Descrição:** Vazamento de dados entre empresas diferentes na mesma instalação.

**Mitigação:**
- Row-level security noSQLite
- Filtragem explícita por company_id
- Testes de isolamento
- Constraints de integridade referencial
- Auditoria de acesso

---

### 3.5 Prompt injection

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Médio ⚠️ |
| **Impacto** | Alto ⚠️ |
| **Prioridade** | Alta |

**Descrição:** Injeção de prompts maliciosos poderia manipular o comportamento dos agentes.

**Mitigação:**
- Templates de prompt padronizados
- Sanitização de inputs
- Limites de comprimento
- Detecção de padrões suspeitos
- Confirmação do proprietário para ações críticas

---

## 4. Riscos Operacionais

### 4.1 Corrupção do banco de dados

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Baixo ⚠️ |
| **Impacto** | Alto ⚠️ |
| **Prioridade** | Média-Alta |

**Descrição:** Corrupção do arquivo SQLite poderia causar perda de dados.

**Mitigação:**
- WAL mode habilitado
- Backup automático configurável
- Verificação de integridade ao iniciar
- Operações atômicas
- Recovery procedures documentadas

---

### 4.2 Problemas de atualização

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Médio ⚠️ |
| **Impacto** | Médio ⚠️ |
| **Prioridade** | Média |

**Descrição:** Atualizações do schema ou do próprio aplicativo poderiam quebrar dados existentes.

**Mitigação:**
- Sistema de migrações versionado
- Testes de upgrade/downgrade
- Backup automático antes de migração
- Compatibilidade reversa onde possível
- Rollback simplificado

---

## 5. Riscos de UI/UX

### 5.1 Complexidade para o proprietário

| Métrica | Valor |
|---------|-------|
| **Probabilidade** | Alto ⚠️ |
| **Impacto** | Médio ⚠️ |
| **Prioridade** | Média |

**Descrição:** Interface muito complexa pode confundir o proprietário.

**Mitigação:**
- Design iterativo com feedback
- Tutorial de onboarding
- Interface progressiva (funcionalidades desbloqueadas)
- Documentação clara
- Suporte a uso comandos via CLI também

---

## 6. Matriz Resumida de Risco

| Risco | Prob. | Impacto | Prioridade | Status |
|-------|-------|---------|------------|--------|
| Curva de aprendizado Rust | Alto | Médio | Média | Monitoreando |
| Limitações SQLite | Médio | Médio | Média-Alta | Mitigado (VectorStore Rust) |
| Complexidade async | Médio | Alto | Média-Alta | Mitigado (padrões) |
| Integração NVIDIA API | Alto | Alto | Alta | Mitigado (segurança) |
| Exposição credenciais | Baixo | Crítico | Alta | Mitigado (Credential Manager) |
| Acesso não autorizado | Baixo | Crítico | Alta | Em desenvolvimento |
| Execução maliciosa | Médio | Crítico | Alta | Mitigado (sandbox) |
| Cross-company leak | Baixo | Crítico | Alta | Mitigado (isolamento) |
| Prompt injection | Médio | Alto | Alta | Mitigado (templates) |
| Corrupção DB | Baixo | Alto | Média-Alta | Mitigado (WAL) |
| Problemas de update | Médio | Médio | Média | Plano definido |
| Complexidade UI | Alto | Médio | Média | Em progresso |

---

## 7. Monitoramento de Risco

Os riscos serão revisados:
- A cada sprint (desenvolvimento)
- Antes de cada release (QA)
- Mensalmente (revisão geral)

---

## 8. Contatos

Para questões de segurança: security@tina.local  
Para questões técnicas: tech@tina.local  
Para reportar bugs: issues@tina.local

---

*Este documento será atualizado conforme novos riscos forem identificados ou mitigações forem implementadas.*