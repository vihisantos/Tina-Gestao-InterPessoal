-- Tina Platform — schema inicial SQLite
-- Arquivo consolidado de migration/001_initial/sql/20261117_001_initial_schema.sql
-- Compatível com SQLite 3 e executável mais de uma vez (IF NOT EXISTS).

-- INSTALLATIONS -------------------------------------------------------------
CREATE TABLE IF NOT EXISTS installations (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    root_path TEXT NOT NULL UNIQUE,
    version TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    owner_uuid TEXT NOT NULL,
    config TEXT
);

-- COMPANIES -----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS companies (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    installation_id TEXT NOT NULL REFERENCES installations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'suspended')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(installation_id, name)
);

CREATE INDEX IF NOT EXISTS idx_companies_installation ON companies(installation_id);

-- DEPARTMENTS ----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS departments (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    code TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'suspended')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(company_id, code)
);

CREATE INDEX IF NOT EXISTS idx_departments_company ON departments(company_id);
CREATE INDEX IF NOT EXISTS idx_departments_code ON departments(code);
CREATE INDEX IF NOT EXISTS idx_departments_status ON departments(status);

-- POSITIONS -----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS positions (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    level INTEGER NOT NULL DEFAULT 0 CHECK (level >= 0),
    authority_level INTEGER NOT NULL DEFAULT 0 CHECK (authority_level >= 0),
    description TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'suspended')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(department_id, name)
);

CREATE INDEX IF NOT EXISTS idx_positions_department ON positions(department_id);
CREATE INDEX IF NOT EXISTS idx_positions_level ON positions(level);
CREATE INDEX IF NOT EXISTS idx_positions_auth ON positions(authority_level);
CREATE INDEX IF NOT EXISTS idx_positions_status ON positions(status);

-- EMPLOYEES -----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS employees (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    position_id TEXT NOT NULL REFERENCES positions(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    personality TEXT,
    specialty TEXT,
    responsibilities TEXT,
    supervisor_id TEXT REFERENCES employees(id) ON DELETE SET NULL,
    model_config TEXT NOT NULL,
    authority_override INTEGER DEFAULT 0 CHECK (authority_override IN (0, 1)),
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(position_id, name)
);

CREATE INDEX IF NOT EXISTS idx_employees_position ON employees(position_id);
CREATE INDEX IF NOT EXISTS idx_employees_supervisor ON employees(supervisor_id);
CREATE INDEX IF NOT EXISTS idx_employees_active ON employees(is_active);

-- DELEGATIONS ----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS delegations (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    delegante_id TEXT NOT NULL REFERENCES employees(id) ON DELETE CASCADE,
    delegado_id TEXT NOT NULL REFERENCES employees(id) ON DELETE CASCADE,
    parent_delegation_id TEXT REFERENCES delegations(id) ON DELETE SET NULL,
    task_description TEXT NOT NULL,
    parameters TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'completed', 'paused', 'interrupted', 'cancelled', 'failed')),
    priority INTEGER DEFAULT 0,
    due_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    result TEXT,
    tools_used TEXT,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_delegations_delegante ON delegations(delegante_id);
CREATE INDEX IF NOT EXISTS idx_delegations_delegado ON delegations(delegado_id);
CREATE INDEX IF NOT EXISTS idx_delegations_status ON delegations(status);
CREATE INDEX IF NOT EXISTS idx_delegations_parent ON delegations(parent_delegation_id);
CREATE INDEX IF NOT EXISTS idx_delegations_due_at ON delegations(due_at);

-- MEMORIES ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    employee_id TEXT REFERENCES employees(id) ON DELETE CASCADE,
    company_id TEXT REFERENCES companies(id) ON DELETE CASCADE,
    department_id TEXT REFERENCES departments(id) ON DELETE CASCADE,
    scope TEXT NOT NULL CHECK (scope IN ('personal', 'professional', 'organizational')),
    category TEXT NOT NULL,
    key TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT,
    source TEXT,
    is_encrypted INTEGER DEFAULT 0 CHECK (is_encrypted IN (0, 1)),
    UNIQUE(employee_id, key)
);

CREATE INDEX IF NOT EXISTS idx_memories_employee ON memories(employee_id);
CREATE INDEX IF NOT EXISTS idx_memories_company ON memories(company_id);
CREATE INDEX IF NOT EXISTS idx_memories_department ON memories(department_id);
CREATE INDEX IF NOT EXISTS idx_memories_scope ON memories(scope);
CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
CREATE INDEX IF NOT EXISTS idx_memories_key ON memories(key);

-- EVENTS --------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    installation_id TEXT NOT NULL REFERENCES installations(id) ON DELETE CASCADE,
    employee_id TEXT REFERENCES employees(id) ON DELETE SET NULL,
    delegation_id TEXT REFERENCES delegations(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'info' CHECK (severity IN ('info', 'warning', 'error', 'critical')),
    payload TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_events_installation ON events(installation_id);
CREATE INDEX IF NOT EXISTS idx_events_employee ON events(employee_id);
CREATE INDEX IF NOT EXISTS idx_events_delegation ON events(delegation_id);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_severity ON events(severity);

-- TOOLS ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tools (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    installation_id TEXT NOT NULL REFERENCES installations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    tool_type TEXT NOT NULL CHECK (tool_type IN ('builtin', 'mcp', 'custom')),
    command TEXT,
    parameters_schema TEXT,
    permissions_required TEXT,
    is_enabled INTEGER NOT NULL DEFAULT 1 CHECK (is_enabled IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(installation_id, name)
);

CREATE INDEX IF NOT EXISTS idx_tools_installation ON tools(installation_id);
CREATE INDEX IF NOT EXISTS idx_tools_type ON tools(tool_type);

-- SESSIONS ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    employee_id TEXT NOT NULL REFERENCES employees(id) ON DELETE CASCADE,
    title TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    UNIQUE(employee_id, id)
);

CREATE INDEX IF NOT EXISTS idx_sessions_employee ON sessions(employee_id);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at);

-- MESSAGES ------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT,
    tokens INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    tool_calls TEXT
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_role ON messages(role);

-- CONFIG --------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY,
    value TEXT,
    type TEXT NOT NULL CHECK (type IN ('string', 'integer', 'boolean', 'json')),
    description TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- TIMESTAMP TRIGGERS --------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS update_employees_updated_at
    AFTER UPDATE ON employees
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE employees SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_delegations_updated_at
    AFTER UPDATE ON delegations
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE delegations SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_memories_updated_at
    AFTER UPDATE ON memories
    FOR EACH ROW
    WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE memories SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- VIEWS ---------------------------------------------------------------------
CREATE VIEW IF NOT EXISTS active_employees AS
SELECT
    e.id,
    e.name,
    e.description,
    p.name AS position_name,
    d.name AS department_name,
    c.name AS company_name,
    e.model_config,
    e.authority_override,
    e.created_at
FROM employees e
JOIN positions p ON e.position_id = p.id
JOIN departments d ON p.department_id = d.id
JOIN companies c ON d.company_id = c.id
WHERE e.is_active = 1;

CREATE VIEW IF NOT EXISTS employee_hierarchy AS
SELECT
    e.id AS employee_id,
    e.name AS employee_name,
    p.name AS position_name,
    p.level AS hierarchy_level,
    supervisor.id AS supervisor_id,
    supervisor.name AS supervisor_name,
    e.created_at
FROM employees e
JOIN positions p ON e.position_id = p.id
LEFT JOIN employees supervisor ON e.supervisor_id = supervisor.id;
