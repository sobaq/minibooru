CREATE TYPE POST_VOTE AS ENUM ('like', 'dislike');

CREATE TABLE user_votes (
    user_id UUID      NOT NULL,
    post_id INTEGER   NOT NULL,
    vote    POST_VOTE NOT NULL,

    FOREIGN KEY (user_id) REFERENCES users ON DELETE CASCADE,
    FOREIGN KEY (post_id) REFERENCES posts ON DELETE CASCADE,
    UNIQUE (user_id, post_id)
);