/** Registro retornado pelo comando de leitura autorizado do Tauri. */
export type EmployeeDirectoryEntry = {
  id: string;
  name: string;
  positionId: string;
  position: string;
  departmentId: string;
  department: string;
  companyId: string;
  company: string;
  active: boolean;
  supervisor?: string | null;
  responsibilities?: string | null;
};
