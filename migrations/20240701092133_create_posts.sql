CREATE TYPE MEDIA_TYPE AS ENUM ('image', 'video');

CREATE TABLE posts (
    id             INTEGER     GENERATED ALWAYS AS IDENTITY,
    uploader_id    UUID     /* NULL = anonymous user */,
    md5            TEXT        NOT NULL,
    width          INTEGER     NOT NULL,
    height         INTEGER     NOT NULL,
    source         TEXT        NOT NULL DEFAULT '',
    uploaded_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    media_type     MEDIA_TYPE  NOT NULL,
    file_size      BIGINT      NOT NULL,
    media_path     TEXT        NOT NULL,
    thumbnail_path TEXT        NOT NULL,

    PRIMARY KEY (id),
    FOREIGN KEY (uploader_id) REFERENCES users,
    UNIQUE      (md5)
);
