# AGENTS.md — Instruções para agentes de IA

Este arquivo é a fonte canônica sobre **quem é a Tina** e **como a identidade visual
do aplicativo deve ser tratada**. Leia antes de alterar UI, ícones, assets ou textos
de apresentação.

---

## 1. A Tina é um personagem — e o personagem é o produto

Tina **não é um placeholder, nem um codinome genérico**. Ela é a mascote oficial do
aplicativo e a identidade visual vinculada ao produto:

- **Aparência:** criatura rosa, corpo arredondado e macio (estilo massinha/3D),
  sem boca, dois olhos ovais pretos, dois bracinhos erguidos e dois pés curtos.
  Ela flutua — a arte oficial a mostra em levitação, como se estivesse comemorando.
- **Cor:** rosa é a cor dela. O app inteiro (fundo, bordas, botões, textos de
  destaque) deriva do rosa da personagem sobre fundo branco.
- **Papel:** ela é a cara da Tina, a agente de gestão interpessoal. Quando o usuário
  vê o app, vê a Tina. Quando vê o ícone na área de trabalho, vê a Tina.

Sempre que escrever documentação, tela, mensagem ou material de apresentação,
trate a personagem como parte do escopo — não como decoração opcional.

## 2. Assets oficiais (não recriar, não substituir)

| Asset | Caminho | Uso |
|---|---|---|
| Arte da personagem (fundo transparente) | `src-frontend/static/tina.png` | Logo do cabeçalho, mascote das telas, favicon |
| Ícone do aplicativo (tile rosa arredondado) | `src-tauri/icons/icon.png` | Ícone 512px |
| Ícone Windows | `src-tauri/icons/icon.ico` | **Obrigatório** para o `tauri-build` gerar o recurso do Windows; se sumir, o build quebra |
| Tamanhos do bundle | `src-tauri/icons/{16,24,32,48,64,128,256}x{...}.png`, `128x128@2x.png` | Listados em `src-tauri/tauri.conf.json` |
| Favicon web | `src-frontend/static/favicon.ico` | Evita 404 na janela Tauri |

Regras:

- **Nunca** recolorir, redesenhar, distorcer, recortar ou substituir a personagem por
  ícone genérico, emoji ou letra ("T").
- **Nunca** trocar o rosa por azul, roxo ou qualquer outra paleta.
- O ícone mantém o enquadramento: tile quadrado de cantos arredondados, fundo
  `#fff7fa` → `#ffe2ed`, borda `#ffc7db`, personagem ocupando ~70% da altura, centralizada.
- A arte tem origem fornecida pelo proprietário. Não baixe, gere ou importe outra
  imagem de mascote.

## 3. Tema — branco + rosa, sem modo escuro

Os tokens vivem em **`src-frontend/src/theme.css`** e são carregados uma única vez por
`src-frontend/src/routes/+layout.svelte`.

Nunca escreva cor hexadecimal solta em componente novo: use `var(--tina-*)`.

| Token | Valor | Uso |
|---|---|---|
| `--tina-pink` | `#ee6495` | Rosa principal |
| `--tina-pink-strong` | `#d94f80` | Rosa forte / hover |
| `--tina-pink-soft` | `#f7a8c4` | Detalhes suaves |
| `--tina-pink-deep` | `#c9266b` | Texto de ênfase |
| `--tina-bg` / `--tina-surface` | `#ffffff` | Fundo branco |
| `--tina-surface-tint` | `#fff7fa` | Chip / código / realce |
| `--tina-surface-pink` | `#ffe9f1` | Badge ativo |
| `--tina-border` | `#ffe0ec` | Bordas |
| `--tina-border-strong` | `#ffc7db` | Borda em destaque |
| `--tina-text` | `#4b2b36` | Texto |
| `--tina-text-muted` | `#7a5a66` | Texto secundário |
| `--tina-gradient-page` | brilho radial rosa sobre branco | Fundo da página |
| `--tina-gradient-button` | `#ee6495` → `#d94f80` | Botões |

**Proibido:** reintroduzir o tema escuro antigo (`#0d1117`, `#161b22`, `#30363d`,
texto `#e6edf3`) ou o azul antigo (`#4f8cff`, `#7b61ff`, `#79c0ff`). Se um reset do
ambiente reverter `theme.css`, `+layout.svelte`, `app.html` ou `+page.svelte`, restaure
a versão versionada — não redesenhe.

## 4. Componentes de interface

Reutilize, não duplique. Local: `src-frontend/src/lib/components/`.

| Componente | Props |
|---|---|
| `Logo.svelte` | `size` (padrão `3.75rem`), `alt` (padrão `Tina`) |
| `Card.svelte` | `title`, snippet opcional `badge`, `children` |
| `Badge.svelte` | `tone: 'default' \| 'ok'`, `children` |
| `Button.svelte` | `type`, `onclick`, `children` |

- Cards/badges/botões **não** devem reimplementar estilo na página.
- Snippets definidos na página podem manter CSS de página (`.value`, `.path`, `.detail`,
  `.error`, `code`).
- A mascote aparece no cabeçalho (`Logo`) e dentro da tela (`.mascot` no bloco `.intro`,
  7,5rem, animação flutuante que respeita `prefers-reduced-motion`).

## 5. Contexto do projeto que você não pode violar

- **Clean-room:** nada de código, asset ou documento de origem não registrada. Ver
  `CONTRIBUTING.md` e `docs/provenance/`.
- **Windows-only**, **local-first**: sem serviço remoto obrigatório; dados em
  `%APPDATA%\tina\tina.db`.
- **NVIDIA NIM** é o único provedor de modelos autorizado.
- Nada de commit direto em `main`: use branch `feat/<id>-<slug>` e PR.
- Documentação de governança e UI em **português (Brasil)**; identificadores técnicos
  podem ficar em inglês.

## 6. Comandos

```bash
npm install                # uma vez, na raiz
npm run dev                # Tauri dev (janela desktop + hot reload)
npm run frontend:check     # svelte-kit sync + svelte-check
npm run frontend:build     # vite build (adapter-static)
```

Antes de concluir qualquer mudança de interface, rode `npm run frontend:check` e
`npm run frontend:build`. Não afirme que passou sem executar.

## 7. Checklist rápido de UI

- [ ] Fundo branco, destaques rosa, nada de tema escuro.
- [ ] Personagem presente e reconhecível (cabeçalho e/ou mascote).
- [ ] Tokens `--tina-*` em vez de hex solto.
- [ ] `Card`/`Badge`/`Button`/`Logo` reutilizados.
- [ ] `icon.ico` presente em `src-tauri/icons/`.
- [ ] `npm run frontend:check` e `npm run frontend:build` verdes.
