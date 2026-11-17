# Especificação funcional — Requisitos aprovados

**Status:** v2 — revisão após aprovação estrutural do bootstrap pelo proprietário.
**Origem desta especificação:** decisões expressas do proprietário do produto. Nenhuma outra
fonte foi utilizada.
**Natureza:** comportamento esperado do produto, verificável por critério de aceite. Não descreve
arquitetura, módulos, dados, linguagem ou organização interna.

Identificadores R-n. Alterar esta especificação exige aprovação explícita do proprietário e
registro em ADR (`docs/decisions/`).

## R-1. Plataforma

**R-1.1** O produto, nesta etapa, opera exclusivamente em Windows.
*Aceite:* o produto funciona em versão suportada de Windows; nenhuma funcionalidade é implementada
para outras plataformas nesta etapa.

## R-2. Local-first

**R-2.1** O produto funciona integralmente offline, incluindo sua funcionalidade principal.
*Aceite:* instalação, cadastro, entrada e operação sem conexão de rede funcionam.

**R-2.2** Nenhum servidor próprio, domínio público ou hospedagem externa é obrigatório.
*Aceite:* remover qualquer dependência externa de uma instalação não impede o uso do produto.

**R-2.3** Todos os dados do usuário permanecem na máquina onde o produto roda, sob controle do usuário.
*Aceite:* o usuário identifica onde seus dados estão armazenados e como removê-los; não há fluxo
automático de dados para fora da máquina sem ação e consentimento do usuário.

## R-3. Provedor de IA

**R-3.1** Nesta etapa, a NVIDIA é o único provedor de IA autorizado para funcionalidades que
requisitam um modelo de IA.
*Aceite:* não existe integração com outro provedor nesta etapa; chamadas ao provedor são
explícitas e dependem de autorização do usuário.

**R-3.2** Identificadores, chaves e material de acesso ao provedor nunca ficam no repositório.
*Aceite:* o repositório não contém segredos; material permanece no dispositivo do usuário.

## R-4. Empresas e departamentos isolados

**R-4.1** O produto suporta múltiplas empresas na mesma instalação, com isolamento entre os dados
das companhias.
*Aceite:* memória, documentos, histórico, credenciais e identidade de uma empresa não podem ser
lidos, misturados ou operados em contexto de outra empresa.

**R-4.2** Cada empresa possui departamentos independentes, com fronteiras próprias de dados e de
responsabilidade.
*Aceite:* um departamento não acessa dados de outro departamento sem autorização explícita da
autoridade competente.

**R-4.3** O isolamento vale dentro e fora da própria empresa: departamentos também isolam dados,
histórico e agentes entre si por padrão.
*Aceite:* operações de um departamento são invisíveis aos demais sem permissão explícita registrada.

## R-5. Hierarquia de agentes e autoridade

**R-5.1** Empresa: a empresa é a unidade organizacional máxima abaixo da instalação. Cada empresa
possui departamentos; cada departamento possui funcionários (agentes) organizados em hierarquia.

**R-5.2** Hierarquia de agentes: agentes ocupam posições em uma hierarquia flexível dentro de
cada departamento, com níveis de comando e responsabilidade. Postos possíveis incluem, sem
limitação de profundidade, agente base, líder de time, chefe de seção, chefe de departamento e
chefes de chefes (níveis executivos sucessivos, com abrangência crescente).
*Aceite:* a hierarquia é expressa, visível e consultada pelos agentes ao decidir autonomia de ação;
não existe profundidade máxima fixa de chefia.

**R-5.3** Delegação: tarefas fluem pela hierarquia de cima para baixo — o superior delega ao
subordinado competente, e pode delegar a alguém mais abaixo quando necessário, sem limite de
aninhamento.
*Aceite:* toda delegação registra, no mínimo: quem delegou, quem recebeu, o quê, quando, prazo,
estado atual, ferramentas utilizadas e o resultado final.

**R-5.4** Tina Central: a instalação possui uma camada executiva de coordenação — a Tina Central —
responsável por despachar, priorizar e ordenar a hierarquia de agentes da instalação. A Tina
Central está **abaixo** da autoridade do proprietário.
*Aceite:* a Tina Central organiza e dirige agentes; toda e qualquer determinação do proprietário
prevalece sobre decisão da Tina Central.

**R-5.5** Autoridade máxima do proprietário: o proprietário é a autoridade máxima absoluta da
instalação, acima da Tina Central e de todos os chefes e chefes de chefes.
*Aceite:* nenhuma decisão de agente ou da Tina Central sobrescreve comando do proprietário.

**R-5.6** Acesso direto: o proprietário tem acesso direto a qualquer funcionário (agente) da
instalação, em qualquer empresa, departamento ou nível hierárquico, sem intermediação obrigatória.
*Aceite:* o proprietário pode falar com, orientar, interromper, pausar, revogar e redirecionar
qualquer agente diretamente.

**R-5.7** Chefes de chefes e hierarquia flexível: a existência de múltiplos níveis de chefia não
restringe os direitos do proprietário nem a comunicação direta; a hierarquia organiza a operação
padrão, nunca a autoridade do proprietário.

## R-6. Comando Central e eventos reais

**R-6.1** O produto oferece uma visão consolidada do que está acontecendo agora — o Comando
Central — mostrando os eventos reais da instalação em tempo real ou próximo do real: quais
tarefas estão em andamento, em que estado, quem as executa e o que foi concluído recentemente.
*Aceite:* o proprietário abre o Comando Central e vê o estado real da operação sem consultar
nada além da instalação local.

**R-6.2** Os eventos reais incluem, no mínimo: delegações criadas, iniciadas, concluídas,
pausadas, interrompidas e canceladas; alertas de bloqueio; conclusão de etapas; avisos e erros
relevantes de execução.
*Aceite:* cada tipo de comando relevante do produto produz um evento visível no Comando Central.

## R-7. Delegações, ferramentas e histórico

**R-7.1** Delegações: todo ato de delegação é registrado (delegante, delegado, tarefa, parâmetros,
prazo, estado, resultado) e permanece consultável.
*Aceite:* o histórico de delegações de um agente mostra quem delegou o quê, quando, com que
resultado.

**R-7.2** Ferramentas utilizadas: cada agente registra quais ferramentas utilizou, com que
parâmetros relevantes e quais saídas produziram, por delegação.
*Aceite:* o histórico de uma delegação mostra as ferramentas utilizadas e seus resultados.

**R-7.3** Histórico e resultados: delegante, delegado e superiores na hierarquia consultam o
resultado de qualquer delegação que atravessou suas instâncias; o proprietário consulta todas.

## R-8. Controle de execução (pausa, interrupção, cancelamento)

**R-8.1** Todo agente em execução de delegação pode ser **pausado** pelo proprietário ou por
superior habilitado, sem destruir o trabalho em andamento.
*Aceite:* pausar interrompe a atividade do agente naquele ponto; retomar continua de onde parou.

**R-8.2** Toda delegação pode ser **interrompida** (o trabalho corrente é encerrado e devolvido
ao delegante) e **cancelada** (a delegação é descartada, com motivo registrado).
*Aceite:* interromper e cancelar produzem eventos no Comando Central e atualizam o estado da
delegação no histórico.

**R-8.3** Pausa, interrupção e cancelamento valem também para a Tina Central quando ela estiver
coordenando (a pausa de uma coordenação suspende o despacho seguinte, sem corromper delegações
registradas).
*Aceite:* pausar, interromper e cancelar funcionam também sobre a coordenação, com os mesmos
efeitos de estado.

## R-9. Tina Auth (autenticação própria)

**R-9.1** O produto possui sua própria camada de identidade, autenticação, sessão e autorização,
independente de serviço externo.
*Aceite:* uma instalação sem qualquer serviço externo suporta cadastro e entrada locais.

**R-9.2** Toda sessão de uso é autenticada; não existe modo de uso "sem login".
*Aceite:* não é possível operar o produto sem se autenticar nesta instalação.

**R-9.3** Credenciais nunca são armazenadas em texto aberto.
*Aceite:* o armazenamento local aplica hash seguro de credenciais e protege material de sessão e
credenciais persistentes.

**R-9.4** Identificadores únicos distinguem usuários e instalações; usuários de uma instalação
não são confundidos com usuários de outra.
*Aceite:* inspeção do armazenamento local mostra identificadores distintos por instalação e
isolamento dos usuários.

**R-9.5** Cada instalação funciona de forma independente; sincronização entre computadores não é
implementada nesta etapa.
*Aceite:* nenhuma rota, serviço ou fluxo de sincronização multi-PC existe nesta etapa.

## R-10. Memória, documentos, histórico e segurança local

**R-10.1** O produto oferece memória persistente aos agentes, armazenada localmente.
*Aceite:* a memória persiste entre sessões; o usuário consegue inspecionar e remover dados.

**R-10.2** Documentos e histórico de trabalho são armazenados localmente sob controle do usuário.
*Aceite:* documentos sobrevivem a reinícios; o caminho de armazenamento é conhecido pelo usuário.

**R-10.3** Segurança e auditoria locais: o produto mantém registros locais suficientes para revisão
pelo proprietário de ações sensíveis, delegações executadas, ferramentas utilizadas e acessos.
*Aceite:* o proprietário consegue revisar ações relevantes sem qualquer serviço externo.

## R-11. Autoridade máxima do proprietário

**R-11.1** O proprietário é, por construção, a autoridade máxima da instalação: acima da Tina
Central, de todos os chefes e da hierarquia inteira.
*Aceite:* toda ação automatizada é auditável e subordinada ao proprietário; existem canais e
registros para revisão, pausa, interrupção, cancelamento e desautorização.

## R-12. Governança de requisitos

**R-12.1** Qualquer requisito novo, alterado ou removido exige aprovação do proprietário
registrada em ADR e atualização deste arquivo na mesma mudança.

**R-12.2** Um PR que declara implementar um requisito sem registrar o critério de aceite atendido
não é mergeado.
