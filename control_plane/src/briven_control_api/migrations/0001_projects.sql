CREATE TABLE IF NOT EXISTS briven_control.projects (
    id text PRIMARY KEY,
    organization_id text NOT NULL,
    name text NOT NULL,
    main_branch_id text NOT NULL,
    state text NOT NULL CHECK (state IN ('provisioning', 'ready', 'failed')),
    UNIQUE (organization_id, name)
);
