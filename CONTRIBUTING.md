# Como contribuir com a Tina

A Tina é reconstruída do zero (*clean-room*) sob responsabilidade exclusiva do proprietário.
Antes de escrever qualquer linha, entenda a regra central do projeto:

> **Origem verificável ou nada.** Nenhum material cuja origem não esteja registrada e aprovada
> entra no repositório — nem como código, nem como teste, nem como documentação, nem como
> configuração, nem "só como referência".

## 1. Fluxo obrigatório

1. Uma decisão é registrada (requisito R-ID ou ADR) antes de virar código.
2. Uma branch é criada a partir de `main`: `feat/<id>-<slug>`, `fix/<id>-<slug>`, `docs/<slug>`,
   `chore/<slug>`.
3. Mudanças pequenas, uma preocupação por PR.
4. Pull Request para `main` com: o que mudou, por que, como verificar, e a seção de proveniência
   preenchida (ver template).
5. Aprovação explícita do proprietário. Nenhuma alteração direta em `main`.
6. Merge somente com as verificações obrigatórias verdes.
7. Proveniência atualizada no mesmo PR quando entrar componente novo.

## 2. O que nunca pode contribuir

- Código, testes, migrações, schemas, manifests, lockfiles ou configurações de uma implementação
  anterior ou de qualquer terceiro incompatível.
- Adaptações "reescritas de memória" cujo objetivo seja reproduzir o resultado dessa implementação.
- Arquivos, dados, binários, modelos, fontes ou imagens sem registro de origem e licença.
- Segredos, credenciais, chaves, arquivos de dados locais, capturas de produção.
- Conteúdo protegido de terceiros sem permissão expressa.

Um pedido de exceção não existe como categoria geral. Se algo parecer aproveitável, pare e pergunte
ao proprietário antes de tocar no arquivo.

## 3. Uso de assistência de IA

- É permitido, desde que o autor declare no PR que a contribuição foi gerada ou refinada com
  assistência de IA, e assuma integral responsabilidade pela revisão humana do resultado.
- A revisão humana é obrigatória: origem, licença, correção, adequação e estilo são conferidos
  por você, não pelo modelo.
- Nenhuma saída de modelo substitui a análise de licença ou a decisão do proprietário.

## 4. Registro de proveniência

Todo componente novo (dependência, licença, ativo, ferramenta) entra em
`docs/provenance/REGISTER.md` no mesmo PR, com: origem, URL, versão, SPDX, método de obtenção,
verificação de integridade e uma linha do porquê. Nenhum registro, nenhuma entrada.

## 5. Disciplina de código e documentação

- Idioma: português (Brasil) para documentação de governança e interface de usuário; identificadores
  técnicos podem usar inglês quando a comunidade os consolidar.
- Commits claros, no imperativo, com escopo: `feat(auth): descreve a mudança`.
- Nada de comentários supérfluos; comentários explicam o porquê, nunca o óbvio.
- Não introduza dependências sem necessidade demonstrada; toda dependência pesa para sempre.
- Não harmonize com estilo externo que conflite com estas regras.

## 6. Comportamento

Respeito, transparência e rastreabilidade acima de velocidade. Dúvidas não se resolvem por pressa;
resolvem-se por decisão do proprietário, registrada.

## 7. Reportar um problema de origem

Se você suspeitar que algo no repositório veio de origem não aprovada, não abra uma issue pública:
use o canal privado de [`SECURITY.md`](SECURITY.md) e informe o caminho do arquivo e o que observou.

## 8. Identidade de commit

O repositório usa a identidade local configurada pelo proprietário para este projeto. Contribuidores
estabelecem seu próprio `user.name` e `user.email` na configuração local do repositório — nunca a
configuração global da máquina por conveniência.
