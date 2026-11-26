import { fireEvent, render } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ChatHistory from '../src/lib/components/ChatHistory.svelte';

describe('ChatHistory', () => {
  it('follows new content, preserves reading position and jumps on request', async () => {
    let resize: (() => void) | undefined;
    const disconnect = vi.fn();
    vi.stubGlobal('ResizeObserver', class {
      constructor(callback: () => void) { resize = callback; }
      observe() {}
      disconnect() { disconnect(); }
    });

    const view = render(ChatHistory, { props: { messages: [] } });
    try {
      const viewport = view.getByRole('log');
      let height = 1000;
      let top = 0;
      Object.defineProperties(viewport, {
        scrollHeight: { configurable: true, get: () => height },
        clientHeight: { configurable: true, get: () => 400 },
        scrollTop: {
          configurable: true,
          get: () => top,
          set: (value: number) => { top = Math.max(0, Math.min(value, height - 400)); }
        }
      });

      expect(resize).toBeTypeOf('function');
      resize?.();
      expect(viewport.scrollTop).toBe(600);
      await fireEvent.scroll(viewport);
      height = 1200;
      resize?.();
      expect(viewport.scrollTop).toBe(800);

      viewport.scrollTop = 200;
      await fireEvent.scroll(viewport);
      height = 1500;
      resize?.();
      expect(viewport.scrollTop).toBe(200);

      await fireEvent.click(view.getByRole('button', { name: /ir para a última mensagem/i }));
      expect(viewport.scrollTop).toBe(1100);
      height = 1600;
      resize?.();
      expect(viewport.scrollTop).toBe(1200);
      expect(view.queryByRole('button', { name: /ir para a última mensagem/i })).toBeNull();
    } finally {
      view.unmount();
      vi.unstubAllGlobals();
    }
    expect(disconnect).toHaveBeenCalledOnce();
  });
});
