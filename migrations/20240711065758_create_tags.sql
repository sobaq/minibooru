CREATE TABLE tag_categories (
    id          INTEGER GENERATED ALWAYS AS IDENTITY,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL DEFAULT '',
    colour      TEXT    NOT NULL,
    rank        INTEGER NOT NULL DEFAULT 0,

    PRIMARY KEY (id),
    UNIQUE (name)
);

CREATE TABLE tags (
    id          INTEGER GENERATED ALWAYS AS IDENTITY,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL DEFAULT '',
    category    INTEGER NOT NULL,

    PRIMARY KEY (id),
    FOREIGN KEY (category) REFERENCES tag_categories,
    UNIQUE (name)
);

CREATE TABLE post_tags (
    post_id INTEGER NOT NULL,
    tag_id  INTEGER NOT NULL,

    FOREIGN KEY (post_id) REFERENCES posts ON DELETE CASCADE,
    FOREIGN KEY (tag_id)  REFERENCES tags  ON DELETE CASCADE,
    UNIQUE (post_id, tag_id)
);