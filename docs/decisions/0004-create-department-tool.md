# ADR 0004 — Contexto pessoal e primeira ferramenta administrativa

**Estado:** proposta em implementação, sujeita à revisão do proprietário.
**Requisitos:** R-4 (isolamento), R-7.2 (ferramentas e histórico).

## Decisão

O proprietário revisou a regra inicial: a vida pessoal será representada por uma organização, normalmente chamada Pessoal, com tipo explícito `personal`; as organizações empresariais têm tipo `business`. Todos os departamentos continuam vinculados a uma organização real no banco, sem exceção de chave estrangeira. A organização pessoal não representa pessoa jurídica. Há uma organização pessoal por instalação. Registros preexistentes não são reclassificados pelo nome.

A Tina deve diferenciar esses contextos, resolver a organização pela identidade persistida e pedir esclarecimento se o destino for ambíguo. A criação é autorizada explicitamente pelo proprietário, com validação e auditoria locais.

## Entrega e critérios

- Migração aditiva e classificação exposta pelo backend e cadastro administrativo.
- Preservar isolamento e auditoria na criação de departamentos.
- Integrar function calling NVIDIA, validação dos argumentos, autorização explícita, tratamento de erros/repetições, resposta com resultado persistido e atualização da interface.
- Validar CI e testes nativos Windows antes do merge.

A primeira etapa não ativa ferramentas no chat. O cadastro e a semântica pessoal são a base para a integração posterior nesta mesma PR.

## Integração do chat

A rodada NVIDIA anuncia somente `create_department`. Uma proposta persistida requer confirmação local explícita; a decisão, o departamento e a mensagem de resultado são transacionais e idempotentes. A organização precisa existir e pode ser cadastrada na aba Organizações. Não há criação implícita de empresa. A compatibilidade real do modelo NVIDIA e a validação Windows permanecem pendentes.
