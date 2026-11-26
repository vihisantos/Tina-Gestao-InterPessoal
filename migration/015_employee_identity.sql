-- O UUID existente é a matrícula canônica; não criar identidade paralela.
CREATE TRIGGER employee_identity_immutable
BEFORE UPDATE OF id ON employees
WHEN NEW.id <> OLD.id
BEGIN
    SELECT RAISE(ABORT, 'employee identity is immutable');
END;
CREATE TRIGGER employee_identity_preserve
BEFORE DELETE ON employees
BEGIN
    SELECT RAISE(ABORT, 'deactivate employee instead of deleting identity');
END;
