import { fireEvent, render } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import EmployeeProfile from '../src/lib/components/EmployeeProfile.svelte';

const profile = {
  name: 'Ana Silva',
  position: 'Analista',
  department: 'Operações',
  company: 'Empresa de teste',
  active: true
};

describe('EmployeeProfile', () => {
  it('exibe identidade, organização e status fornecidos pelas props', () => {
    const view = render(EmployeeProfile, {
      props: { ...profile, supervisor: 'João', responsibilities: 'Planejamento' }
    });
    for (const value of [profile.name, profile.position, profile.department,
      profile.company, 'Ativo', 'João', 'Planejamento']) {
      expect(view.getByText(value)).toBeTruthy();
    }
    expect(view.container.querySelector('.avatar')?.textContent).toBe('AS');
  });

  it('não inventa supervisor, responsabilidades ou ação de abertura', () => {
    const view = render(EmployeeProfile, { props: { ...profile, active: false } });
    expect(view.getByText('Inativo')).toBeTruthy();
    expect(view.queryByText('Supervisor')).toBeNull();
    expect(view.queryByText('Responsabilidades')).toBeNull();
    expect(view.queryByRole('button')).toBeNull();
  });

  it('encaminha abertura do perfil apenas ao callback fornecido', async () => {
    const onopen = vi.fn();
    const view = render(EmployeeProfile, { props: { ...profile, onopen } });
    await fireEvent.click(view.getByRole('button', { name: 'Ver perfil de Ana Silva' }));
    expect(onopen).toHaveBeenCalledTimes(1);
  });

  it('atualiza dados e status quando as props mudam', async () => {
    const view = render(EmployeeProfile, { props: profile });
    await view.rerender({ ...profile, name: 'Érica Souza', active: false });
    expect(view.getByText('Érica Souza')).toBeTruthy();
    expect(view.getByText('Inativo')).toBeTruthy();
    expect(view.queryByText('Ana Silva')).toBeNull();
    expect(view.container.querySelector('.avatar')?.textContent).toBe('ÉS');
  });

  it('trata texto de perfil como dado, não como HTML executável', () => {
    const value = '<img src=x onerror=alert(1)>';
    const view = render(EmployeeProfile, { props: { ...profile, responsibilities: value } });
    expect(view.getByText(value)).toBeTruthy();
    expect(view.container.querySelector('img')).toBeNull();
  });
});
