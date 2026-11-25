import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import MarkdownContent from '../src/lib/components/MarkdownContent.svelte';
import MermaidDiagram from '../src/lib/components/MermaidDiagram.svelte';

const { initializeMock, renderMock } = vi.hoisted(() => ({
  initializeMock: vi.fn(),
  renderMock: vi.fn()
}));

vi.mock('mermaid', () => ({
  default: {
    initialize: initializeMock,
    render: renderMock
  }
}));

beforeEach(() => {
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

    await waitFor(() => expect(view.container.querySelector('svg')).not.toBeNull());

    expect(initializeMock).toHaveBeenCalledWith(expect.objectContaining({ securityLevel: 'strict' }));
    expect(view.container.querySelector('script')).toBeNull();
    expect(view.container.querySelector('[onclick]')).toBeNull();
    expect(view.container.querySelector('a')).toBeNull();
    expect(view.container.querySelector('use')?.getAttribute('href')).toBe('#seta');
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
    await waitFor(() => expect(view.container.querySelector('svg')).not.toBeNull());
  });
});
