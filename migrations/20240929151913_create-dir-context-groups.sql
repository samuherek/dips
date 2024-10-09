-- Add migration script here
CREATE TABLE context_groups (
    id text not null primary key,
    name text not null,
    created_at timestamp not null,
    dir_context_id text,
    constraint fk_dir_context
        foreign key (dir_context_id)
        references dir_contexts(id)
);

ALTER TABLE dips ADD COLUMN context_group_id TEXT;

CREATE TABLE new_dips (
    id text NOT NULL PRIMARY KEY,
    value TEXT NOT NULL,
    note TEXT,
    created_at TIMESTAMP NOT NULL, 
    updated_at TIMESTAMP NOT NULL,
    context_group_id TEXT,
    dir_context_id text,
    constraint fk_context_group FOREIGN KEY (context_group_id) REFERENCES context_groups(id),
    constraint fk_dir_context foreign key (dir_context_id) references dir_contexts(id)
);

INSERT INTO new_dips (id, value, note, created_at, updated_at, context_group_id, dir_context_id)
    SELECT id, value, note, created_at, updated_at, context_group_id, dir_context_id FROM dips;

DROP TABLE dips;

ALTER TABLE new_dips RENAME TO dips;
