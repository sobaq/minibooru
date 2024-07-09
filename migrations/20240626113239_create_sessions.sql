CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE sessions (
    token      UUID        DEFAULT uuid_generate_v4(),
    user_id    UUID        NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (token),
    FOREIGN KEY (user_id) REFERENCES users ON DELETE CASCADE
);
