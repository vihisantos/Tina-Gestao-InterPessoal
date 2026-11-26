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
- Integridade: arquivos versionados; testes nativos pendentes.
- Por que: R-4 e R-7.2.
- Incorporado por: branch feat/R-7.2-create-department-tool, sujeita a revisão.

## RH de agentes — ADR 0005
- Origem: decisão do proprietário e implementação original assistida por IA neste repositório.
- Licença: proprietária do repositório; revisão humana pendente.
- Obtenção: código original; nenhuma dependência externa nova.
- Integridade: arquivos versionados; testes nativos pendentes.
- Por que: R-5.8 e R-7.2.
- Incorporado por: branch feat/R-5-rh-agent-hiring, sujeita a revisão.

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
