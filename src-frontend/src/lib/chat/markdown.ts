import MarkdownIt from 'markdown-it';
import DOMPurify from 'dompurify';

export type RenderedMarkdown = {
  html: string;
  diagrams: string[];
};

type RenderOptions = {
  renderDiagrams?: boolean;
};

type FenceEnv = {
  diagramMode: 'code' | 'placeholder';
  diagrams: string[];
};

const markdown = new MarkdownIt({
  html: false,
  linkify: false,
  breaks: false,
  typographer: false
});

markdown.disable(['html_block', 'html_inline']);

type Purifier = ReturnType<typeof DOMPurify>;

let htmlPurifier: Purifier | undefined;
let svgPurifier: Purifier | undefined;

function getPurifiers(): { html: Purifier; svg: Purifier } | null {
  if (typeof window === 'undefined') return null;

  if (!htmlPurifier) {
    htmlPurifier = DOMPurify(window);
    htmlPurifier.addHook('beforeSanitizeElements', (node) => {
      if (!(node instanceof Element)) return;
      if (['SCRIPT', 'IFRAME', 'OBJECT', 'EMBED', 'FORM', 'LINK', 'META', 'BASE'].includes(node.tagName)) {
        node.remove();
      }
    });
    htmlPurifier.addHook('afterSanitizeAttributes', (node) => {
      if (!(node instanceof Element)) return;
      for (const attribute of ['src', 'srcset', 'poster', 'formaction', 'srcdoc']) {
        if (node.hasAttribute(attribute)) node.removeAttribute(attribute);
      }
      const href = node.getAttribute('href');
      if (href !== null && !/^https?:\/\//i.test(href.trim())) {
        node.removeAttribute('href');
        node.setAttribute('aria-disabled', 'true');
        node.setAttribute('tabindex', '-1');
      }
      const style = node.getAttribute('style');
      if (style && /(url\s*\(|expression\s*\(|@import|javascript:|data:text\/html)/i.test(style)) {
        node.removeAttribute('style');
      }
    });
  }

  if (!svgPurifier) {
    svgPurifier = DOMPurify(window);
    svgPurifier.addHook('beforeSanitizeElements', (node) => {
      if (!(node instanceof Element)) return;
      if (node.tagName.toLowerCase() === 'style') {
        const cleaned = (node.textContent ?? '').replace(
          /url\s*\(\s*(['"]?)([^)'"]+)\1\s*\)/gi,
          (whole, _quote: string, value: string) => (value.trim().startsWith('#') ? whole : '')
        );
        if (!cleaned.trim()) node.remove();
        else node.textContent = cleaned;
      }
    });
    svgPurifier.addHook('afterSanitizeAttributes', (node) => {
      if (!(node instanceof Element)) return;
      for (const attribute of ['href', 'xlink:href']) {
        const value = node.getAttribute(attribute);
        if (value !== null && !value.trim().startsWith('#')) node.removeAttribute(attribute);
      }
      for (const attribute of ['src', 'srcset', 'poster', 'formaction', 'srcdoc']) {
        if (node.hasAttribute(attribute)) node.removeAttribute(attribute);
      }
      const style = node.getAttribute('style');
      if (style && /(url\s*\(|expression\s*\(|@import|javascript:|data:text\/html)/i.test(style)) {
        node.removeAttribute('style');
      }
    });
  }

  return { html: htmlPurifier, svg: svgPurifier };
}

function sanitizeHtml(html: string): string {
  const purifier = getPurifiers();
  if (!purifier) return html;
  return purifier.html.sanitize(html, {
    USE_PROFILES: { html: true },
    ALLOW_DATA_ATTR: true,
    ADD_ATTR: ['target'],
    FORBID_TAGS: ['script', 'iframe', 'object', 'embed', 'form', 'link', 'meta', 'base']
  });
}

export function sanitizeMermaidSvg(svg: string): string {
  const purifier = getPurifiers();
  if (!purifier) return '';
  return purifier.svg.sanitize(svg, {
    USE_PROFILES: { html: true, svg: true, svgFilters: true },
    ADD_TAGS: ['foreignObject', 'use'],
    FORBID_TAGS: ['script', 'iframe', 'object', 'embed', 'form', 'link', 'meta', 'base', 'a', 'img', 'audio', 'video', 'source'],
    FORBID_ATTR: ['srcdoc'],
    ALLOW_DATA_ATTR: false
  });
}

markdown.renderer.rules.link_open = (tokens, index, options, _env, self) => {
  const token = tokens[index];
  const href = token.attrGet('href') ?? '';
  if (/^https?:\/\//i.test(href.trim())) {
    token.attrSet('target', '_blank');
    token.attrSet('rel', 'noopener noreferrer nofollow');
    token.attrSet('class', 'tina-safe-link');
  } else {
    token.attrSet('href', '#');
    token.attrSet('rel', 'noopener noreferrer nofollow');
    token.attrSet('aria-disabled', 'true');
    token.attrSet('tabindex', '-1');
    token.attrSet('class', 'tina-blocked-link');
    token.attrSet('title', 'Link bloqueado');
  }
  return self.renderToken(tokens, index, options);
};

markdown.renderer.rules.image = (tokens, index) => {
  const token = tokens[index];
  const alt = token.content || token.attrGet('alt') || '';
  return alt ? markdown.utils.escapeHtml(alt) : '';
};

markdown.renderer.rules.fence = (tokens, index, options, env, self) => {
  const token = tokens[index];
  const info = token.info.trim().split(/\s+/)[0]?.toLowerCase() ?? '';
  const language = info.replace(/[^a-z0-9_+-]/gi, '');
  const code = markdown.utils.escapeHtml(token.content);

  if (info === 'mermaid' && (env as FenceEnv).diagramMode === 'placeholder') {
    const position = (env as FenceEnv).diagrams.push(token.content) - 1;
    return `<div class="tina-mermaid-placeholder" data-mermaid-index="${position}">Gerando diagrama…</div>\n`;
  }

  const languageClass = language ? ` class="language-${language}"` : '';
  return [
    '<div class="tina-code-wrap">',
    '<button type="button" class="tina-copy-button" data-copy-code aria-label="Copiar código">Copiar</button>',
    `<pre class="tina-code-block"><code${languageClass}>${code}</code></pre>`,
    '</div>\n'
  ].join('');
};

export function renderMarkdown(content: string, options: RenderOptions = {}): RenderedMarkdown {
  const diagrams: string[] = [];
  const env: FenceEnv = {
    diagramMode: options.renderDiagrams ? 'placeholder' : 'code',
    diagrams
  };
  const html = markdown.render(content ?? '', env);
  return { html: sanitizeHtml(html), diagrams };
}
