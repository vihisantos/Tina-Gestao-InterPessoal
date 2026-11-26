# ADR 0005 — RH e contratação de agentes de IA

**Estado:** implementação incremental, sujeita à aprovação do proprietário.
**Requisitos:** R-3, R-4, R-5.8, R-7.2, R-10.3 e R-11.
**Origem:** decisão do proprietário de que Tina identifique necessidades de contratação e utilize ferramentas de RH para o contrato e a escolha de IA.

## Decisão

A Tina deve tratar contratação como um processo, não como um cadastro isolado. Um agente é um funcionário de IA; o contrato é um documento operacional interno, não um vínculo trabalhista humano.

1. Identificar necessidade concreta e consultar agentes/cargos existentes antes de criar.
2. Propor cargo, organização, departamento, hierarquia, atribuições e justificativa. Aprovação cria o cargo, não o funcionário. Por padrão, um cargo novo não concede permissões extras.
3. Elaborar perfil e contrato operacional: identidade, especialidade, responsabilidades, objetivos verificáveis, supervisor, limites, ferramentas autorizadas, acesso a dados, modelo, orçamento e critérios de revisão.
4. Consultar o catálogo de modelos NVIDIA NIM autorizado pela instalação. Excluir modelos indisponíveis ou incompatíveis. Avaliar candidatos nas mesmas tarefas sintéticas, com critérios definidos antes do teste. Registrar resultados observados de qualidade, latência, capacidade de ferramentas e custo quando conhecido. Nunca tratar preço estimado ou pontuação inventada como medição.
5. Preparar proposta de contratação com evidências, alternativas, contrato e limites. Se nenhuma opção satisfizer os requisitos, registrar bloqueio; não contratar com modelo provisório.
6. Somente após autorização explícita, persistir funcionário, contrato versionado e configuração do modelo, com auditoria e isolamento. Permissões de ferramenta continuam aplicadas no backend; o contrato não concede acesso por si só.

## Entrega inicial desta PR

- Tool `create_position` no chat, acionável por iniciativa da Tina, com proposta persistida, aprovação/recusa, auditoria, revalidação de escopo e idempotência.
- Contrato do processo de RH e critérios de aceite para avaliação real e contratação.
- A criação do contrato, a consulta real ao catálogo NVIDIA, os testes comparativos e a efetivação de contratação ainda não estão implementados nesta etapa. Não oferecer ferramentas inexistentes ao modelo.
- A branch depende da PR 73 para reaproveitar o mecanismo de propostas; revisar e mesclar a base primeiro. Nenhuma alteração direta em main.

## Validação

Testar iniciativa sem comando literal, cargo sem aprovação, aprovação e recusa, repetição, duplicidade, escopo e departamento arquivado. Antes de liberar contratação, testar catálogo indisponível, ausência de evidências, modelos incompatíveis, custo desconhecido, persistência atômica e ausência de privilégios implícitos. Validação nativa Windows e NVIDIA permanece necessária.

## Aplicativos e ferramentas atribuíveis (decisão posterior)

A Skill ensina o procedimento de RH; ferramentas executam operações; aplicativos agrupam ferramentas disponíveis. Nesta etapa, RH inclui create_position. Ferramentas de criação de organização/departamento e administração de acessos permanecem centrais. Concessão de pacote é uma fotografia das ferramentas delegáveis no momento da autorização; revogar uma ferramenta remove seu acesso mesmo se recebida pelo pacote. Uma concessão é limitada ao funcionário, organização e departamento; mudança de escopo invalida o acesso anterior. A Tina pode iniciar propostas de atribuição ou revogação; nesta fase a confirmação é do proprietário. O backend nega por padrão e não concede autoridade administrativa. O executor de funcionários ainda não está integrado: a concessão representa acesso configurado, não uma ação executada por um agente. Antes de habilitar a execução por funcionários, o dispatcher deve aplicar o verificador de acesso e a política específica da ação a cada chamada. Contrato e avaliação reais de modelos permanecem pendentes.
