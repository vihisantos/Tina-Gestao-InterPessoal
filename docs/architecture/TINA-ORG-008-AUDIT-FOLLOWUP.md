# TINA-ORG-008 — continuidade da auditoria organizacional

Refs: #8, #17, #32, #33, #52. Base revisada: 1f5042c.

## Estado

A implementação de autoria transacional e consulta autorizada está na main. A PR #17 é substituída como linha de desenvolvimento, sem integrar a versão antiga das migrations. A issue #8 continua aberta. Este documento registra trabalho pendente, não comprova execução de testes nem conclusão da funcionalidade.

## Entregas pendentes

1. Limitar consultas de auditoria e paginação por cursor a 1–200 registros por chamada, rejeitando valores negativos, zero e acima do limite. Preservar a autorização de sessão, instalação e empresa. Testar limites e cursores inválidos.
2. Exercitar escritas realmente concorrentes usando banco temporário, verificando autoria de cada evento, ausência de contexto residual, rollback, isolamento e ausência de evento enganoso de sucesso. Consultar a cobertura anterior preservada na branch da #33.
3. Conferir cobertura de alterações de permissões e transferências organizacionais permitidas. Se houver lacuna de triggers, criar migration nova com versão livre e testes; nunca modificar as migrations 009, 011 ou 012 já aplicadas.
4. Revisar registro de operações negadas, sem atribuir identidade não autenticada nem expor conteúdo sensível.
5. Validar os testes Rust de auditoria e recuperação de migrations em banco temporário. O CI hospedado atual valida governança, formatação Rust e frontend; não comprova execução dos testes nativos.

## Critérios de aceite

- Consultas limitadas e autorizadas, com isolamento e paginação verificáveis.
- Eventos atômicos, imutáveis pela camada de aplicação e com autoria confiável.
- Cobertura de eventos coerente com as operações realmente disponíveis.
- Resultados de testes registrados com o commit correspondente; não declarar aprovação de testes não executados.
- Não alterar banco real, credenciais, migrations antigas ou proteções da main.

As PRs #18, #19 e #20 foram reorganizadas independentemente da #17. A integração desta continuidade não é condição para preservar os respectivos documentos.
