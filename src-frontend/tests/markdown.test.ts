import { describe, expect, it } from 'vitest';
import { renderMarkdown, sanitizeMermaidSvg, isSafeMermaidSource } from '../src/lib/chat/markdown';

describe('renderMarkdown', () => {
  it('renderiza títulos, listas, tabelas, citações, negrito e itálico', () => {
    const source = [
      '# Resumo',
      '',
      '- **Primeiro** ponto',
      '- *Segundo* ponto',
      '',
      '> citação importante',
      '',
      '| Item | Valor |',
      '| --- | --- |',
      '| A | 1 |'
    ].join('\n');

    const { html } = renderMarkdown(source);

    expect(html).toContain('<h1>Resumo</h1>');
    expect(html).toContain('<ul>');
    expect(html).toContain('<li>');
    expect(html).toContain('<strong>Primeiro</strong>');
    expect(html).toContain('<em>Segundo</em>');
    expect(html).toContain('<blockquote>');
    expect(html).toContain('<table>');
    expect(html).toContain('<th>Item</th>');
    expect(html).toContain('<td>1</td>');
  });

  it('transforma URLs web explícitas em links, sem ativar domínios soltos ou código', () => {
    const { html } = renderMarkdown('Visite https://github.com/ e example.com. `https://example.org/`');
    expect(html).toContain('href="https://github.com/"');
    expect(html).toContain('class="tina-safe-link"');
    expect(html).not.toContain('href="http://example.com"');
    expect(html).not.toContain('href="https://example.org/"');
  });

  it('permite apenas links http(s) e bloqueia URLs perigosas', () => {
    const source = [
      '[Site seguro](https://example.com/docs)',
      '[Script](javascript:alert(1))',
      '[Dados](data:text/html,<script>alert(1)</script>)',
      '[FTP](ftp://example.com/arquivo)'
    ].join('\n');

    const { html } = renderMarkdown(source);

    expect(html).toContain('href="https://example.com/docs"');
    expect(html).toContain('target="_blank"');
    expect(html).toContain('rel="noopener noreferrer nofollow"');
    expect(html).not.toMatch(/href=["']javascript:/i);
    expect(html).not.toMatch(/href=["']data:/i);
    expect(html).not.toMatch(/href=["']ftp:/i);
    expect(html).toMatch(/aria-disabled="true"|>\[Script\]\(javascript:alert\(1\)\)</);
    expect(html).not.toMatch(/<script\b/i);
  });

  it('bloqueia HTML arbitrário, scripts e carregamento de imagens externas', () => {
    const source = [
      '<script>alert(1)</script>',
      '',
      '<img src="https://rastreador.example/pixel.png" alt="rastreador">',
      '',
      '![foto](https://externo.example/foto.png)'
    ].join('\n');

    const { html } = renderMarkdown(source);

    expect(html).not.toMatch(/<script\b/i);
    expect(html).not.toMatch(/<img\b/i);
    expect(html).not.toMatch(/<img\b[^>]*src=["']https?:/i);
    expect(html).toContain('foto');
  });

  it('mantém Mermaid como código durante streaming e troca por placeholder quando concluído', () => {
    const source = ['```mermaid', 'graph TD', '  A --> B', '```'].join('\n');

    const streaming = renderMarkdown(source, { renderDiagrams: false });
    expect(streaming.diagrams).toEqual([]);
    expect(streaming.html).toContain('language-mermaid');
    expect(streaming.html).toContain('data-copy-code');

    const complete = renderMarkdown(source, { renderDiagrams: true });
    expect(complete.diagrams).toHaveLength(1);
    expect(complete.diagrams[0]).toContain('A --> B');
    expect(complete.html).toContain('data-mermaid-index="0"');
    expect(complete.html).not.toContain('language-mermaid');
  });
});

describe('sanitizeMermaidSvg', () => {
  it('remove scripts, eventos, links perigosos, imagens externas e CSS externo', () => {
    const dirty = [
      '<svg xmlns="http://www.w3.org/2000/svg">',
      '<style>@import url(https://estilo.example/x.css); .node { fill: url(https://estilo.example/y.png); background-image: url(#ok); }</style>',
      '<script>alert(1)</script>',
      '<g onclick="alert(1)">',
      '<a href="javascript:alert(1)">clique</a>',
      '<image href="https://imagem.example/x.png" />',
      '<image href="#ok" />',
      '</g>',
      '</svg>'
    ].join('');

    const clean = sanitizeMermaidSvg(dirty);

    expect(clean.toLowerCase()).toContain('<svg');
    expect(clean).not.toMatch(/<script\b/i);
    expect(clean).not.toContain('onclick');
    expect(clean).not.toContain('javascript:');
    expect(clean).not.toContain('estilo.example');
    expect(clean).not.toContain('imagem.example');
    expect(clean).not.toMatch(/<a\b/i);
    expect(clean).toContain('href="#ok"');
  });
});

describe('isSafeMermaidSource', () => {
  it('accepts common static diagrams', () => {
    expect(isSafeMermaidSource('flowchart TD\nA[Início] --> B[Fim]')).toBe(true);
    expect(isSafeMermaidSource('sequenceDiagram\nA->>B: Olá')).toBe(true);
  });
  it.each([
    '%%{init: {"securityLevel":"loose"}}%%\ngraph TD\nA-->B',
    '---\nconfig:\n  securityLevel: loose\n---\ngraph TD',
    'graph TD\nclick A "https://example.com"',
    'graph TD\nclassDef a fill:url(https://example.com)',
    'graph TD\nA@{ img: "https://example.com/a.png" }',
    'graph TD\nA["<img src=x>"]',
    'graph TD\n' + 'A'.repeat(20000)
  ])('rejects configuration, active content or oversized input', (source) => {
    expect(isSafeMermaidSource(source)).toBe(false);
  });
});
