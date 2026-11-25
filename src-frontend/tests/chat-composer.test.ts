import { fireEvent, render } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ChatComposer from '../src/lib/components/ChatComposer.svelte';
import ChatMessage from '../src/lib/components/ChatMessage.svelte';
import ChatHistory from '../src/lib/components/ChatHistory.svelte';

describe('chat composer', () => {
  it('sends with Enter, preserves Shift+Enter and ignores IME/repeat', async () => {
    const onSend = vi.fn();
    const view = render(ChatComposer, { props: { draft: 'Olá', onSend, onCancel: vi.fn() } });
    const field = view.getByRole('textbox');
    await fireEvent.keyDown(field, { key: 'Enter', shiftKey: true });
    await fireEvent.keyDown(field, { key: 'Enter', isComposing: true });
    await fireEvent.keyDown(field, { key: 'Enter', repeat: true });
    expect(onSend).not.toHaveBeenCalled();
    await fireEvent.keyDown(field, { key: 'Enter' });
    expect(onSend).toHaveBeenCalledOnce();
  });
  it('does not send when busy, permits cancel only during generation', async () => {
    const onSend = vi.fn(), onCancel = vi.fn();
    const view = render(ChatComposer, { props: { draft: 'Olá', busy: true, onSend, onCancel } });
    expect((view.getByRole('button') as HTMLButtonElement).disabled).toBe(true);
    await view.rerender({ generating: true });
    await fireEvent.click(view.getByRole('button', { name: 'Interromper' }));
    expect(onCancel).toHaveBeenCalledOnce();
    expect(onSend).not.toHaveBeenCalled();
  });
});
describe('response actions', () => {
  it('copies full original response after completion only', async () => {
    const view = render(ChatMessage, { props: { role: 'assistant', content: '**Olá**', streaming: true } });
    expect(view.queryByRole('button', { name: 'Copiar resposta' })).toBeNull();
    await view.rerender({ streaming: false });
    await fireEvent.click(view.getByRole('button', { name: 'Copiar resposta' }));
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('**Olá**');
  });
  it('shows a real generation state even before the first token', async () => {
    const view = render(ChatHistory, { props: { messages: [], generating: true } });
    expect(view.getByText('Tina está preparando a resposta…')).toBeTruthy();
    await view.rerender({ generating: false });
    expect(view.queryByText('Tina está preparando a resposta…')).toBeNull();
  });
});
