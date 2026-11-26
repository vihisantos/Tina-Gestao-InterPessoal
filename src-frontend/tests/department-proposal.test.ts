import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import DepartmentProposal from '../src/lib/components/DepartmentProposal.svelte';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

type Proposal = {
  id: string;
  tool_name: string;
  lifecycle_status: string;
  reason: string;
  company_name: string;
  organization_kind: string;
  name: string;
  code: string;
  description: string | null;
};

const proposal: Proposal = {
  id: 'proposal-1',
  tool_name: 'create_department',
  lifecycle_status: 'pending',
  reason: 'Separar rotina financeira',
  company_name: 'Pessoal',
  organization_kind: 'personal',
  name: 'Finanças',
  code: 'FIN',
  description: null
};

const message = {
  id: 'message-1',
  role: 'assistant',
  content: 'Departamento criado.',
  created_at: '2026-01-01 00:00:00'
};

function props(sessionId = 'session-1', lastMessageId = 'message-0', ondecision = vi.fn()) {
  return { props: { sessionId, lastMessageId, ondecision } };
}

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation((command: string) => {
    if (command === 'chat_pending_department') return Promise.resolve(proposal);
    return Promise.resolve(message);
  });
});

describe('DepartmentProposal', () => {
  it('mostra a proposta pendente com as três decisões disponíveis', async () => {
    const view = render(DepartmentProposal, props());

    await waitFor(() => expect(view.getByText('Finanças')).toBeTruthy());
    expect(view.getByRole('button', { name: 'Confirmar criação' })).toBeTruthy();
    expect(view.getByRole('button', { name: 'Recusar' })).toBeTruthy();
    expect(view.getByRole('button', { name: 'Cancelar proposta' })).toBeTruthy();
    expect(view.getByText(/Aguardando decisão/i)).toBeTruthy();
  });

  it('mantém o cartão visível após novas mensagens e recarrega a proposta', async () => {
    const view = render(DepartmentProposal, props());
    await waitFor(() => expect(view.getByText('Finanças')).toBeTruthy());

    await view.rerender({ lastMessageId: 'message-1' });

    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(2));
    expect(view.getByText('Finanças')).toBeTruthy();
  });

  it('recusa com o id da proposta e devolve a mensagem ao chat', async () => {
    const ondecision = vi.fn();
    const view = render(DepartmentProposal, props('session-1', 'message-0', ondecision));
    await waitFor(() => expect(view.getByText('Finanças')).toBeTruthy());

    await fireEvent.click(view.getByRole('button', { name: 'Recusar' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('chat_decide_department', {
        sessionId: 'session-1',
        proposalId: 'proposal-1',
        approved: false
      })
    );
    await waitFor(() => expect(ondecision).toHaveBeenCalledWith(message));
    await waitFor(() => expect(view.queryByText('Finanças')).toBeNull());
  });

  it('cancela sem criar nada e remove o cartão', async () => {
    const ondecision = vi.fn();
    const view = render(DepartmentProposal, props('session-1', 'message-0', ondecision));
    await waitFor(() => expect(view.getByText('Finanças')).toBeTruthy());

    await fireEvent.click(view.getByRole('button', { name: 'Cancelar proposta' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('chat_cancel_proposal', {
        sessionId: 'session-1',
        proposalId: 'proposal-1'
      })
    );
    await waitFor(() => expect(ondecision).toHaveBeenCalledWith(message));
    expect(view.queryByText('Finanças')).toBeNull();
  });

  it('ignora clique duplo enquanto a decisão está em andamento', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'chat_pending_department') return Promise.resolve(proposal);
      return new Promise(() => undefined);
    });
    const view = render(DepartmentProposal, props());
    await waitFor(() => expect(view.getByText('Finanças')).toBeTruthy());

    const button = view.getByRole('button', { name: 'Confirmar criação' });
    await fireEvent.click(button);
    await fireEvent.click(button);

    const decisions = invokeMock.mock.calls.filter(([command]) => command === 'chat_decide_department');
    expect(decisions).toHaveLength(1);
  });

  it('não deixa resposta antiga sobrescrever a proposta da nova sessão', async () => {
    let resolveFirst: ((value: Proposal) => void) | undefined;
    invokeMock
      .mockImplementationOnce(() => new Promise<Proposal>(resolve => { resolveFirst = resolve; }))
      .mockResolvedValueOnce({ ...proposal, id: 'proposal-2', name: 'Operações' });

    const view = render(DepartmentProposal, props('session-1', 'message-1'));
    await view.rerender({ sessionId: 'session-2', lastMessageId: 'message-2' });
    await waitFor(() => expect(view.getByText('Operações')).toBeTruthy());

    resolveFirst?.(proposal);
    await waitFor(() => expect(view.queryByText('Finanças')).toBeNull());
    expect(view.getByText('Operações')).toBeTruthy();
  });

  it('informa erro do backend sem esconder a proposta', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'chat_pending_department') return Promise.resolve(proposal);
      return Promise.reject(new Error('proposta substituída'));
    });
    const view = render(DepartmentProposal, props());
    await waitFor(() => expect(view.getByText('Finanças')).toBeTruthy());

    await fireEvent.click(view.getByRole('button', { name: 'Confirmar criação' }));

    await waitFor(() => expect(view.getByRole('alert').textContent).toContain('proposta substituída'));
    expect(view.getByText('Finanças')).toBeTruthy();
  });
});
