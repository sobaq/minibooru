CREATE TYPE RESOURCE AS ENUM ('posts', 'wiki');
CREATE TYPE OPERATION AS ENUM ('create', 'read', 'update', 'delete');

CREATE TABLE groups (
    id          INTEGER GENERATED ALWAYS AS IDENTITY,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL DEFAULT '',
    colour      TEXT    NOT NULL DEFAULT '#808080',
    /* A superuser always has permission to do anything */
    superuser   BOOLEAN NOT NULL DEFAULT false,

    PRIMARY KEY (id),
    UNIQUE (name)
);

CREATE TABLE permissions (
    group_id  INTEGER, /* NULL = Anonymous user's group */
    operation OPERATION NOT NULL,
    resource  RESOURCE  NOT NULL,

    FOREIGN KEY (group_id) REFERENCES groups ON DELETE CASCADE,
    UNIQUE NULLS NOT DISTINCT (group_id, operation, resource)
);
