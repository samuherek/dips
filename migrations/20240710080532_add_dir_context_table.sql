-- Add migration script here
CREATE TABLE dir_contexts(
    id VARCHAR(36) NOT NULL PRIMARY KEY,
    dir_path TEXT NOT NULL,
    git_remote TEXT,
    git_dir_name TEXT,
    created_at TIMESTAMP NOT NULL, 
    updated_at TIMESTAMP NOT NULL
);

-- Set the dir_context_id to the original dips
ALTER TABLE dips ADD COLUMN dir_context_id TEXT;

-- Create a new dips table with the foreign key constraint
CREATE TABLE new_dips (
    id VARCHAR(36) NOT NULL PRIMARY KEY,
    value TEXT NOT NULL,
    note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    dir_context_id VARCHAR(36),
    CONSTRAINT fk_dir_context
        FOREIGN KEY (dir_context_id) 
        REFERENCES dir_contexts(id)
);

-- Copy data from the old dips table to the new dips table
INSERT INTO new_dips (id, value, note, created_at, updated_at, dir_context_id)
SELECT id, value, note, created_at, updated_at, dir_context_id FROM dips;

-- Drop the old dips table
DROP TABLE dips;

-- Rename the new dips table to the original table name
ALTER TABLE new_dips RENAME TO dips;
