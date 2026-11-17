# ADR-0003 — Local-first, NVIDIA como único provedor de IA

**Data:** 2026-11-15
**Status:** Aprovada pelo proprietário.

## Contexto

O proprietário definiu soberania de dados como requisito de produto: dados locais, operação
autônoma, sem infraestrutura obrigatória do fornecedor. Também definiu a NVIDIA como único
provedor de IA autorizado nesta etapa.

## Decisão

1. A Tina funciona sem rede. Nenhum servidor, domínio ou serviço próprio do produto é obrigatório.
2. Todos os dados do usuário ficam na máquina onde o produto roda, sob controle do usuário.
3. Nesta etapa, somente a NVIDIA pode ser usada para IA; nenhuma chamada para outro provedor.
4. Material de acesso ao provedor nunca reside no repositório.

## Consequências

- Arquitetura e inicialização devem funcionar sem dependências externas.
- Qualquer comentário ou documento que sugira outro provedor deve ser tratado como violação.
- Expandir provedores exige decisão do proprietário registrada em ADR e atualização das
  especificações na mesma mudança.
