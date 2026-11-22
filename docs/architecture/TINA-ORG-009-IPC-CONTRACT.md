# TINA-ORG-009 — contrato de IPC organizacional (rascunho)

Dependências: #7 autorização e #8 auditoria. Expor empresas, departamentos, cargos, funcionários e hierarquia exclusivamente por comandos Tauri. Resolver o ator no backend, nunca por IDs declarados no payload. DTOs explícitos; validar identificadores, campos, limites e escopos. Erros retornados ao frontend não devem incluir SQL, caminhos locais ou segredos. Preservar `ping` e `bootstrap`.

**Não implementado:** handlers, registro no invoke_handler, integração com serviços, testes IPC e validação CI. Este documento é somente preparação da issue.
