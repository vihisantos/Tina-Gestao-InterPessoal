# ADR-0001 — Reconstrução clean-room

**Data:** 2026-11-15
**Status:** Aprovada pelo proprietário.

## Contexto

O projeto Tina existia anteriormente sob outra organização técnica. O proprietário decidiu
reconstruir a Tina *do zero*, em repositório oficial limpo, com material próprio e origens
verificadas.

## Decisão

A reconstrução segue clean-room: nada daquela implementação é copiado, adaptado, transportado ou
reescrito de memória neste repositório. Os requisitos funcionais já aprovados pelo proprietário
podem ser reescritos neste repositório como especificação independente, sem reproduzir
organização, implementação ou documentação técnica protegida.

## Consequências

- Nenhum arquivo da implementação anterior entra neste repositório.
- Proveniência fica registrada em `docs/provenance/`; integridade em `docs/integrity/`.
- Código futuro é escrito a partir de `docs/spec/000-requisitos-funcionais.md` e das decisões
  do proprietário — nunca a partir do material anterior.
- Qualquer dúvida de origem bloqueia a entrada no repositório até decisão do proprietário.
