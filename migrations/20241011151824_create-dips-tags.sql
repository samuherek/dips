-- Add migration script here
CREATE TABLE tags (
    id text not null primary key,
    name text not null unique,
    created_at timestamp not null
);

INSERT INTO tags (id, name, created_at)
SELECT id, name, created_at FROM context_groups GROUP BY name;

--
CREATE TABLE dips_tags (
    dip_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    PRIMARY KEY (dip_id, tag_id),
    FOREIGN KEY (dip_id) REFERENCES dips(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

--
CREATE TABLE new_dips (
    id text NOT NULL PRIMARY KEY,
    value TEXT NOT NULL,
    note TEXT,
    created_at TIMESTAMP NOT NULL, 
    updated_at TIMESTAMP NOT NULL,
    dir_context_id text,
    constraint fk_dir_context foreign key (dir_context_id) references dir_contexts(id)
);

INSERT INTO new_dips (id, value, note, created_at, updated_at, dir_context_id)
SELECT id, value, note, created_at, updated_at, dir_context_id FROM dips;

DROP TABLE dips;

ALTER TABLE new_dips RENAME TO dips;

DROP TABLE context_groups;
