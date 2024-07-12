CREATE TABLE user_favourites (
    user_id UUID    NOT NULL,
    post_id INTEGER NOT NULL,

    FOREIGN KEY (user_id) REFERENCES users ON DELETE CASCADE,
    FOREIGN KEY (post_id) REFERENCES posts ON DELETE CASCADE,
    UNIQUE (user_id, post_id)
);