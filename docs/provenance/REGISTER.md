# Registro de Proveniência

**Estado atual (bootstrap):** vazio, por projeto. Nenhum componente de terceiros foi
incorporado ainda.

Template de entrada para quando o primeiro componente entrar:

```
## <nome> — <versão>
- Origem: <URL>
- Autoria: <titular dos direitos>
- Licença: <SPDX> — verificação: <data/como>
- Integridade: <checksum/assinatura>
- Por que: <requisito R-ID / decisão>
- Incorporado por: <PR>
- Aviso correspondente: NOTICE (quando exigido por licença)
```

## Contexto pessoal e primeira ferramenta — ADR 0004
- Origem: decisão do proprietário; implementação original assistida por IA neste repositório.
- Licença: proprietária do repositório; revisão humana pendente.
- Obtenção: código original; nenhuma dependência externa nova.
- Integridade: arquivos versionados; migrations 013, 014, 015 e 019 aditivas, sem edição de SQL publicado; checksums e upgrades legados cobertos por testes no Windows.
- Por que: R-4 e R-7.2.
- Incorporado por: branch feat/R-7.2-create-department-tool, PR #73 em rascunho, sujeita a revisão; nenhum merge realizado.

## RH de agentes — ADR 0005
- Origem: decisão do proprietário e implementação original assistida por IA neste repositório.
- Licença: proprietária do repositório; revisão humana pendente.
- Obtenção: código original; nenhuma dependência externa nova.
- Integridade: arquivos versionados; testes nativos pendentes.
- Por que: R-5.8 e R-7.2.
- Incorporado por: branch feat/R-5-rh-agent-hiring, PR #74 em rascunho, sujeita a revisão.

## Integração #73 × #74 — ciclo de vida unificado
- Origem: decisão do proprietário; integração assistida por IA no clone isolado
  da PR #74, sem merge de PRs e sem rebase/force push.
- Obtenção: merge sem commit do head `9db4694` da PR #73 na branch da PR #74,
  preservando o merge commit e o histórico publicado de `9cf1918`.
- Integridade: migration aditiva `020_proposal_lifecycle_rh.sql`; nenhuma
  migration publicada foi editada; banco ativo `%APPDATA%\tina\tina.db` não foi
  alterado; upgrades de `1..20` e de `1..15,19 → 1..20` cobertos por testes.
- Por que: unificar cancelamento, substituição e idempotência entre as cinco
  famílias de propostas sem regredir as ferramentas de RH da PR #74.
- Incorporado por: branch feat/R-5-rh-agent-hiring, PR #74 em rascunho; a PR
  #73 permanece aberta em rascunho.

Entradas futuras vão antes deste parágrafo inicial, em ordem cronológica.

## Avaliação sintética NVIDIA — PR 74
- Implementação original assistida por IA; reutiliza reqwest e o cofre nativo existentes.
- Testes sintéticos de aritmética e chamada de ferramenta simulada, sem dados privados.
- Não é avaliação de aptidão profissional completa; custo desconhecido e contratação ainda bloqueada.
- Revisão humana e validação Rust/Windows e NVIDIA pendentes.

## Minutas operacionais de RH — PR 74
- Implementação original assistida por IA; migração 018 e aprovação pelo chat.
- Minuta não concede ferramentas, não seleciona modelo e não efetiva contratação.
- Revisão humana e testes nativos pendentes.

## Executor experimental de funcionários — PR 74
- Implementação original assistida por IA; usa o transporte NVIDIA existente e escopo SQLite autenticado.
- Ação direta do proprietário; ferramenta de leitura list_positions somente mediante concessão. Sem contratação automática, agendamento, conectores MCP externos ou escrita por funcionário.
- Revisão humana e validação Windows/NVIDIA pendentes.
