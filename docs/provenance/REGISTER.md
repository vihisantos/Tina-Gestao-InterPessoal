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

Entradas futuras vão antes deste parágrafo inicial, em ordem cronológica.
