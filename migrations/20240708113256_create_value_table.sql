-- Add migration script here
CREATE TABLE dips(
    id VARCHAR(36) NOT NULL PRIMARY KEY,
    value TEXT NOT NULL,
    note TEXT,
    created_at TIMESTAMP NOT NULL, 
    updated_at TIMESTAMP NOT NULL
);
