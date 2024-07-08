-- Add migration script here
CREATE TABLE dips(
    id uuid NOT NULL,
    PRIMARY KEY (id),
    value TEXT NOT NULL,
    note TEXT,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL
);
