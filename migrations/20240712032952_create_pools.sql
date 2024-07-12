CREATE TABLE pools (
    id          INTEGER  GENERATED ALWAYS AS IDENTITY,
    name        TEXT     NOT NULL,
    description TEXT     NOT NULL DEFAULT '',
    creator_id  UUID, /* NULL = Anonymous or deleted user */

    PRIMARY KEY (id),
    FOREIGN KEY (creator_id) REFERENCES users ON DELETE SET NULL
);

CREATE TABLE post_pools (
    post_id INTEGER NOT NULL,
    pool_id INTEGER NOT NULL,

    FOREIGN KEY (post_id) REFERENCES posts ON DELETE CASCADE,
    FOREIGN KEY (pool_id) REFERENCES pools ON DELETE CASCADE,
    UNIQUE (post_id, pool_id)
);