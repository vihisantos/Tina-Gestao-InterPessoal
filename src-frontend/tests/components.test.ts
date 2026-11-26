import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import MarkdownContent from '../src/lib/components/MarkdownContent.svelte';
import MermaidDiagram from '../src/lib/components/MermaidDiagram.svelte';

const { initializeMock, renderMock, invokeMock } = vi.hoisted(() => ({
  initializeMock: vi.fn(),
  renderMock: vi.fn(),
  invokeMock: vi.fn()
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

vi.mock('mermaid', () => ({
  default: {
    initialize: initializeMock,
    render: renderMock
  }
}));

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockResolvedValue(undefined);
  renderMock.mockImplementation(async (_id: string, code: string) => {
    if (code.includes('INVALID')) throw new Error('diagrama inválido');
    return {
      svg: [
        '<svg xmlns="http://www.w3.org/2000/svg">',
        '<script>alert(1)</script>',
        '<g onclick="alert(1)"><a href="javascript:alert(1)">clique</a><use href="#seta" /></g>',
        '</svg>'
      ].join('')
    };
  });
});

describe('MermaidDiagram', () => {
  it('usa Mermaid em modo strict e sanitiza o SVG gerado', async () => {
    const view = render(MermaidDiagram, { props: { code: 'graph TD; A --> B;' } });

    await waitFor(() => expect(view.container.querySelector('img')).not.toBeNull());
    const src = view.container.querySelector('img')?.getAttribute('src') ?? '';
    expect(src).toMatch(/^data:image\/svg\+xml/);
    const svg = decodeURIComponent(src.slice(src.indexOf(',') + 1));
    expect(initializeMock).toHaveBeenCalledWith(expect.objectContaining({ securityLevel: 'strict', htmlLabels: false }));
    expect(view.container.querySelector('svg')).toBeNull();
    expect(svg).not.toMatch(/<script|onclick|<a\b/i);
    expect(svg).toContain('href="#seta"');
  });

  it('mostra o código original como fallback para diagramas inválidos', async () => {
    const invalid = 'graph TD\n  INVALID --> ;';
    const view = render(MermaidDiagram, { props: { code: invalid } });

    await waitFor(() =>
      expect(view.getByText(/Não foi possível gerar o diagrama/i)).toBeTruthy()
    );

    expect(view.container.querySelector('svg')).toBeNull();
    expect(view.container.textContent).toContain('INVALID');
  });
});

describe('MarkdownContent', () => {
  it('encaminha links web ao backend sem navegar a WebView', async () => {
    const view = render(MarkdownContent, {
      props: { content: '[GitHub](https://github.com/)' }
    });
    const link = view.getByRole('link', { name: 'GitHub' });
    expect(invokeMock).not.toHaveBeenCalled();
    const event = new MouseEvent('click', { bubbles: true, cancelable: true });
    await fireEvent(link, event);
    expect(event.defaultPrevented).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith('chat_open_link', { url: 'https://github.com/' });
  });

  it('não abre links bloqueados e informa falha de abertura', async () => {
    const view = render(MarkdownContent, {
      props: { content: '[Relativo](/arquivo) [GitHub](https://github.com/)' }
    });
    const blocked = view.container.querySelector('.tina-blocked-link');
    expect(blocked).not.toBeNull();
    await fireEvent.click(blocked!);
    expect(invokeMock).not.toHaveBeenCalled();
    invokeMock.mockRejectedValueOnce(new Error('falha'));
    await fireEvent.click(view.getByRole('link', { name: 'GitHub' }));
    await waitFor(() => expect(view.getByRole('alert')).toBeTruthy());
  });

  it('adiciona botão de copiar aos blocos de código', async () => {
    const view = render(MarkdownContent, {
      props: { content: '```js\nconst resposta = 42;\n```' }
    });

    const button = view.getByRole('button', { name: /copiar código/i });
    await fireEvent.click(button);

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
      expect.stringContaining('const resposta = 42;')
    );
    expect(button.textContent).toContain('Copiado!');
  });

  it('renderiza Mermaid somente quando a resposta estiver concluída', async () => {
    const source = ['```mermaid', 'graph TD', '  A --> B', '```'].join('\n');
    const view = render(MarkdownContent, { props: { content: source, streaming: true } });

    expect(view.container.querySelector('[data-mermaid-index]')).toBeNull();
    expect(view.container.querySelector('code.language-mermaid')).not.toBeNull();

    await view.rerender({ streaming: false });

    await waitFor(() => expect(view.container.querySelector('[data-mermaid-index]')).not.toBeNull());
    await waitFor(() => expect(view.container.querySelector('img')).not.toBeNull());
  });
});

describe('Mermaid source gate', () => {
  it('never invokes renderer for model-supplied initialization directives', async () => {
    renderMock.mockClear();
    const view = render(MermaidDiagram, { props: { code: 'graph TD\n%%{init: {"securityLevel":"loose"}}%%\nA --> B' } });
    await waitFor(() => expect(view.getByText(/Não foi possível gerar o diagrama/i)).toBeTruthy());
    expect(renderMock).not.toHaveBeenCalled();
  });
});
