/* To execute: PGPASSWORD=postgres psql -Upostgres -h0.0.0.0 -f dev/gen-db.sql */
/* CONFIGURATION */
\set post_count 1_000_000
\set user_count 200_000

/* SCRIPT */
SET client_min_messages TO WARNING;
\set group_count (:user_count / 30)

BEGIN;

TRUNCATE groups RESTART IDENTITY CASCADE;
INSERT INTO groups (name, description)
SELECT
    'Group' || generate_series(0, :group_count),
    'A little description of my group';

TRUNCATE users RESTART IDENTITY CASCADE;
INSERT INTO users (group_id, username, password)
SELECT
    floor((random() * :group_count) + 1),
    'User' || generate_series(0, :user_count),
    'Password';

TRUNCATE posts RESTART IDENTITY CASCADE;
INSERT INTO posts (
    uploader_id, md5, width,
    height, media_type, file_size,
    media_path, thumbnail_path)
SELECT
    uploader.id,
    'MD' || post_num,
    random() * 5000, random() * 5000, 'image',
    random() * 10_000_000, 'Media Path ' || post_num,
    'Thumbnail Path ' || post_num
FROM generate_series(0, :post_count) AS post_num,
    /* TODO: Only one random ID is selected then cached by psql */
     (SELECT users.id FROM users ORDER BY random() LIMIT 1) AS uploader;

TRUNCATE user_votes RESTART IDENTITY CASCADE;
INSERT INTO user_votes (user_id, post_id, vote)
SELECT
    usser.id, post_id,
    (CASE WHEN random() > 0.5 THEN 'like' ELSE 'dislike' END)::POST_VOTE
FROM (SELECT users.id FROM users ORDER BY random() LIMIT 1) AS usser,
     generate_series(1, :post_count) AS post_id;

TRUNCATE user_favourites RESTART IDENTITY CASCADE;
INSERT INTO user_favourites (user_id, post_id)
SELECT
    usser.id, post_id
FROM (SELECT users.id FROM users ORDER BY random() LIMIT 1) AS usser,
     generate_series(1, :post_count) AS post_id;

COMMIT;