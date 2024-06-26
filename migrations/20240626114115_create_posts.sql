CREATE TYPE media_type AS ENUM ('image', 'video');
CREATE TYPE safety AS ENUM ('safe', 'questionable', 'explicit');

CREATE TABLE posts (
    id          INTEGER    GENERATED ALWAYS AS IDENTITY,
    uploader_id UUID       NOT NULL,
    md5         TEXT       NOT NULL,
    width       INTEGER    NOT NULL,
    height      INTEGER    NOT NULL,
    safety      SAFETY     NOT NULL,
    source      TEXT       NOT NULL DEFAULT '',
    uploaded_at TIMESTAMP  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted     BOOLEAN    NOT NULL DEFAULT FALSE,
    media_type  MEDIA_TYPE NOT NULL,
    file_size   BIGINT     NOT NULL,

    media_path  TEXT       NOT NULL,
    thumb_path  TEXT       NOT NULL,

    PRIMARY KEY (id),
    FOREIGN KEY (uploader_id) REFERENCES users,
    UNIQUE      (md5)
);
