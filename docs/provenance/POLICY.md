# Política de Proveniência

Nenhuma entrada de componente entra neste repositório sem registro antes ou no mesmo PR.

## O que é proveniência

Origem verificável de tudo que entra: código escrito aqui (autoria: proprietário/contribuidores),
dependências de terceiros, ativos (ícones, fontes, imagens, modelos), ferramentas de build e
automations, e decisões derivadas de requisitos aprovados pelo proprietário.

## O que o registro contém

Cada componente registrado lista:

- **Nome e versão** (com checksum quando aplicável)
- **Origem** (URL do registro/fonte e autoria)
- **Licença** (SPDX) e verificação do titular dos direitos
- **Integridade** (checksum, assinatura ou método de verificação)
- **Por que** (necessidade demonstrado pelo requisito ou decisão aprovada)

## Origem de primeira parte

Código escrito para este repositório tem origem registrada como: autoria do proprietário/
contribuidores declarados, licenciado sob a decisão de licenciamento pendente
(ver `LICENSING.md`).

## Consequências de ausência de registro

Um PR com componente sem registro falha a verificação de proveniência e não é mergeado. O registro
segue `REGISTER.md`; template de entrada abaixo.

## Obrigação de manutenção

`REGISTER.md` é a responsabilidade de quem introduz o componente: o mesmo PR que introduz a
dependência introduz o registro. Bumps de versão repetem a verificação mínima (versão, SPDX,
checksum). Remoções apagam a entrada e atualizam `NOTICE`.
