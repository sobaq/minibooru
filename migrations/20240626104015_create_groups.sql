CREATE TYPE RESOURCE AS ENUM ('posts', 'wiki');
CREATE TYPE OPERATION AS ENUM ('create', 'read', 'update', 'delete');

CREATE TABLE groups (
    id          INTEGER GENERATED ALWAYS AS IDENTITY,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL,
    default     BOOLEAN NOT NULL,

    PRIMARY KEY (id),
    UNIQUE (name)
)

CREATE TABLE permissions (
    group_id  INTEGER   NOT NULL,
    operation OPERATION NOT NULL,
    resource  RESOURCE  NOT NULL,

    FOREIGN KEY (group_id) REFERENCES users ON DELETE CASCADE,
    UNIQUE (group_id, operation, resource)
);
