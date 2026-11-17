# ADR-0002 — Windows exclusivo nesta etapa

**Data:** 2026-11-15
**Status:** Aprovada pelo proprietário.

## Contexto

A Tina é destinada a empresas que operam predominantemente em Windows. Outras plataformas não
foram aprovadas e não devem ser presumidas.

## Decisão

O produto é exclusivamente Windows nesta etapa. Nenhuma implementação, build ou empacotamento para
outra plataforma será feito sem decisão registrada de mudança de escopo.

## Consequências

- Distribuição, build e testes existem somente para Windows.
- Nenhuma abstração de plataforma é construída "por antecipação" nesta etapa.
- Mudanças de escopo exigem ADR e aprovação explícita do proprietário.
