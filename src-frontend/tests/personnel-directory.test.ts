import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import PersonnelDirectory from '../src/lib/components/PersonnelDirectory.svelte';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));
const row = { id: 'employee-1', name: 'Pessoa de teste', positionId: 'p', position: 'Analista',
  departmentId: 'd', department: 'Operações', companyId: 'c', company: 'Empresa',
  active: false, supervisor: null, responsibilities: 'Responsabilidade de teste' };

beforeEach(() => invokeMock.mockReset());

describe('PersonnelDirectory', () => {
  it('carrega somente os registros retornados pelo IPC e permite abrir o perfil', async () => {
    invokeMock.mockResolvedValue([row]);
    const view = render(PersonnelDirectory);
    await waitFor(() => expect(view.getByText(row.name)).toBeTruthy());
    expect(invokeMock).toHaveBeenCalledWith('list_employee_directory');
    expect(view.getByText('Inativo')).toBeTruthy();
    await fireEvent.click(view.getByRole('button', { name: 'Ver perfil de Pessoa de teste' }));
    expect(view.getByRole('button', { name: 'Voltar à lista' })).toBeTruthy();
    expect(view.getByText(row.responsibilities)).toBeTruthy();
    await fireEvent.click(view.getByRole('button', { name: 'Voltar à lista' }));
    expect(view.getByRole('searchbox')).toBeTruthy();
  });

  it('não vaza erro bruto e permite recarregar sem dados fictícios', async () => {
    invokeMock.mockRejectedValueOnce(new Error('erro interno SQLite')).mockResolvedValueOnce([]);
    const view = render(PersonnelDirectory);
    await waitFor(() => expect(view.getByRole('alert')).toBeTruthy());
    expect(view.queryByText(/erro interno SQLite/)).toBeNull();
    await fireEvent.click(view.getByRole('button', { name: 'Atualizar funcionários' }));
    await waitFor(() => expect(view.getByText('Nenhum funcionário cadastrado.')).toBeTruthy());
  });
});
