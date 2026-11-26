import { fireEvent, render } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import EmployeeSpace from '../src/lib/components/EmployeeSpace.svelte';

const employees = [
  { id: 'a', name: 'Álvaro Dias', position: 'Analista', department: 'Operações', company: 'Empresa A', active: true },
  { id: 'b', name: 'Beatriz Lima', position: 'Gerente', department: 'Financeiro', company: 'Empresa A', active: false },
  { id: 'c', name: 'Caio Souza', position: 'Analista', department: 'Operações', company: 'Empresa B', active: true }
];

describe('EmployeeSpace', () => {
  it('agrupa os funcionários fornecidos por empresa e departamento', () => {
    const view = render(EmployeeSpace, { props: { employees } });
    expect(view.getByRole('heading', { name: 'Empresa A' })).toBeTruthy();
    expect(view.getByRole('heading', { name: 'Empresa B' })).toBeTruthy();
    expect(view.getAllByRole('heading', { name: 'Operações' })).toHaveLength(2);
    expect(view.getByRole('heading', { name: 'Financeiro' })).toBeTruthy();
    expect(view.getByRole('status').textContent).toContain('3 de 3');
  });

  it('busca sem distinguir acentos ou maiúsculas', async () => {
    const view = render(EmployeeSpace, { props: { employees } });
    await fireEvent.input(view.getByRole('searchbox', { name: 'Buscar' }), { target: { value: 'ALVARO' } });
    expect(view.getByRole('heading', { name: 'Álvaro Dias' })).toBeTruthy();
    expect(view.queryByRole('heading', { name: 'Beatriz Lima' })).toBeNull();
    expect(view.queryByRole('heading', { name: 'Caio Souza' })).toBeNull();
  });

  it('combina filtros de empresa e status', async () => {
    const view = render(EmployeeSpace, { props: { employees } });
    await fireEvent.change(view.getByRole('combobox', { name: 'Empresa' }), { target: { value: 'Empresa A' } });
    await fireEvent.change(view.getByRole('combobox', { name: 'Status' }), { target: { value: 'inactive' } });
    expect(view.getByRole('heading', { name: 'Beatriz Lima' })).toBeTruthy();
    expect(view.queryByRole('heading', { name: 'Álvaro Dias' })).toBeNull();
    expect(view.queryByRole('heading', { name: 'Caio Souza' })).toBeNull();
    expect(view.getByRole('status').textContent).toContain('1 de 3');
  });

  it('distingue carregamento, falha e ausência de dados', async () => {
    const view = render(EmployeeSpace, { props: { employees: [], loading: true } });
    expect(view.getByRole('status').textContent).toContain('Carregando');
    await view.rerender({ employees: [], loading: false, error: 'Falha de teste' });
    expect(view.getByRole('alert').textContent).toBe('Falha de teste');
    await view.rerender({ employees: [], loading: false, error: undefined });
    expect(view.getByText('Nenhum funcionário cadastrado.')).toBeTruthy();
  });

  it('informa quando nenhum funcionário corresponde à busca', async () => {
    const view = render(EmployeeSpace, { props: { employees } });
    await fireEvent.input(view.getByRole('searchbox', { name: 'Buscar' }), { target: { value: 'inexistente' } });
    expect(view.getByText('Nenhum funcionário corresponde aos filtros.')).toBeTruthy();
  });

  it('encaminha a identidade do funcionário selecionado ao callback', async () => {
    const onopen = vi.fn();
    const view = render(EmployeeSpace, { props: { employees, onopen } });
    await fireEvent.click(view.getByRole('button', { name: 'Ver perfil de Beatriz Lima' }));
    expect(onopen).toHaveBeenCalledTimes(1);
    expect(onopen).toHaveBeenCalledWith('b');
  });
});
