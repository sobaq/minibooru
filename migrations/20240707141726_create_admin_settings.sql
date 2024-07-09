CREATE TABLE users (
    id         UUID        DEFAULT uuid_generate_v4(),
    group      INTEGER     NOT NULL,
    username   TEXT        NOT NULL,
    password   TEXT        NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (id),
    FOREIGN KEY (group) REFERENCES groups,
    CONSTRAINT username_unique UNIQUE (username),
    CONSTRAINT non_empty CHECK (username <> '')
);
