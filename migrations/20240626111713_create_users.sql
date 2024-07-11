CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE users (
    id         UUID        NOT NULL DEFAULT uuid_generate_v4(),
    group_id   INTEGER     NOT NULL,
    username   TEXT        NOT NULL,
    password   TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (id),
    FOREIGN KEY (group_id) REFERENCES groups,
    CONSTRAINT username_unique UNIQUE (username),
    CONSTRAINT non_empty CHECK (username <> '')
);
