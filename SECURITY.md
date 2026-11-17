# Política de segurança

A Tina é um projeto privado, em fase de bootstrap, sem usuários externos. Ainda assim, esta
política já vale desde o primeiro commit.

## Reportando uma vulnerabilidade

**Nunca** abra issue, discussão ou PR público com detalhes de segurança.

Decisões de canal:

- Preferencialmente, use **GitHub Security Advisories** privados do repositório:
  `https://github.com/vihisantos/Tina-Gestao-InterPessoal/security/advisories/new`
- Se o Advisory não estiver disponível para a sua conta, contate o proprietário pelo canal
  privado da equipe. Não divulgue o problema por nenhum outro meio.

Inclua: o que foi encontrado, onde (caminho do arquivo, commit), impacto estimado, e como
reproduzir. Não inclua exploits prontos em material público depois de corrigido.

## O que está dentro do escopo agora

- Vazamento de segredos no histórico, no CI ou nos fluxos de contribuição.
- Violação da política de integridade: material de origem não aprovada no repositório.
- Qualquer código futuro com comportamento malicioso ou imprevisto introduzido por dependência.

## O que ainda não existe

- Código de aplicação, então superfícies "clássicas" (rede, input remoto, autenticação) ainda não se
  aplicam. Quando existirem, esta seção será atualizada no mesmo PR que as introduzir.

## Compromissos permanentes

- **Segredos nunca entram no repositório.** CI falha fechado em padrões de chave privada.
- **Dados locais permanecem locais.** Nenhuma telemetria é coletada sem decisão explícita do
  proprietário, com consentimento visível para o usuário final.
- **Integridade de origem é controle de segurança.** Um vazamento de código de origem não aprovada
  é tratado como incidente, não como "detalhe jurídico".

## Prazos e expectativas

Este é um projeto de um mantenedor, em desenvolvimento. Não há SLA contratual. Os relatórios são
analisados assim que possível, e você receberá confirmação de recebimento pelo mesmo canal privado.

## Reconhecimento

Agradecemos relatórios responsáveis. Com a sua autorização, o reconhecimento é feito no release que
incorporou a correção; nunca antes.

## Fora de escopo desta política

Questões de licença de terceiros entram em [`CONTRIBUTING.md`](CONTRIBUTING.md) e no registro de
proveniência — o canal de segurança trata de risco técnico e de acesso, e permanece estritamente
privado.
