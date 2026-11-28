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

## Ferramentas e comandos implementados

- `create_department` (function calling NVIDIA): propõe um departamento com `organization_kind`, `company_name` opcional, `name`, `code`, `description` e `reason`. Não grava nada durante a inferência.
- `create_organization` (function calling NVIDIA): propõe uma organização `personal` ou `business` com `reason`. Não cria departamentos automaticamente.
- `owner_pending_department`: consulta a proposta ativa da sessão, sem efeito de escrita.
- `owner_decide_department`: confirma ou recusa a proposta; grava entidade, auditoria e mensagem de resultado na mesma transação do proprietário.
- `owner_cancel_proposal` e o comando Tauri `chat_cancel_proposal`: cancelam a proposta sem criar nada.
- `repair_migration_checksums`: ferramenta explícita, fora da inicialização, para divergência comprovadamente apenas `LF`/`CRLF`, com backup e confirmação.

A resolução da organização usa `kind`, nunca o nome: um cadastro comercial chamado “Pessoal” não é a organização pessoal. O código rejeita argumentos desconhecidos, campos forjados de autoridade, nomes/códigos inválidos, duplicatas e alvos ambíguos, e limita tudo à `installation_id` autenticada.

## Ciclo de vida das propostas

`lifecycle_status` é a fonte autoritativa (`pending`, `approved`, `rejected`, `superseded`, `cancelled`); `status` permanece legado para leitores antigos. Mensagens posteriores não escondem a proposta pendente. Uma nova proposta substitui a anterior na mesma sessão e instalação, marcando-a como `superseded` com `superseded_by`; aprovação ou recusa da proposta substituída falha. Decisão repetida com o mesmo resultado devolve a mesma mensagem persistida; decisão oposta, cancelamento de proposta decidida e aprovação de proposta cancelada falham sem gravar. Cancelamento repetido é idempotente e gera auditoria uma única vez.

O cartão do chat mostra a justificativa e oferece Confirmar criação, Recusar e Cancelar proposta. Ele recarrega quando a conversa avança, sem apagar a proposta, e descarta respostas antigas quando a sessão muda.

## Migrations desta branch

| Versão | Arquivo | Finalidade |
|---:|---|---|
| 13 | `013_company_kind.sql` | tipo explícito `personal`/`business`, no máximo uma organização pessoal por instalação. |
| 14 | `014_chat_department_proposals.sql` | propostas de departamento persistidas por sessão. |
| 15 | `015_proactive_organization_proposals.sql` | propostas de organização por iniciativa da Tina. |
| 16 | `016_chat_position_proposals.sql` | propostas de cargo, sem contratação automática. |
| 17 | `017_tool_app_grants.sql` | propostas de atribuição e revogação de ferramentas. |
| 18 | `018_chat_contract_drafts.sql` | minutas operacionais aguardando autorização. |
| 19 | `019_proposal_lifecycle.sql` | estados explícitos, substituição segura, cancelamento e índices de uma proposta ativa por sessão. |
| 20 | `020_proposal_lifecycle_rh.sql` | ciclo de vida unificado das cinco famílias de propostas após a integração das PRs #73 e #74. |

As versões 16 a 18 vieram da PR #74 e a 20 foi adicionada na integração, sem
rebase nem force push. As migrations publicadas não foram editadas; toda mudança
de schema é aditiva e forward-only. O upgrade de bancos legados é validado a
partir das próprias SQLs incorporadas, preservando dados e falhando em
divergência real, versão futura ou migration marcada como falha. Nunca há reparo
automático na inicialização.

## Validação e pendências

Foram executados `cargo fmt --all -- --check`, `cargo check --all-targets`, `cargo test`, `npm run frontend:check`, `npm run frontend:test`, `npm run frontend:build` e `cargo build -p tina-desktop` no Windows. A compatibilidade com o modelo NVIDIA real e a validação manual do proprietário na janela Tauri continuam pendentes antes de qualquer merge.
