use std::{os::unix::fs::MetadataExt, path::{Path, PathBuf}};
use anyhow::Context;
use argon2::password_hash::rand_core::{self, RngCore};
use ffmpeg::{codec::{self, context::Context as CodecContext}, frame::Video, software::scaling::context::Context as ScalingContext, Rescale};
use askama_axum::IntoResponse;
use axum::{extract::{self, multipart::Field, Multipart, State}, routing::{get, post}, Json, Router};
use hex::ToHex;
use md5::Digest;
use serde::{Deserialize, Serialize};
use tokio::{fs::{self, File}, io::AsyncWriteExt};
use infer::MatcherType;
use uuid::Uuid;

use crate::{
    extractors::{Authentication, Operation::*, Permission, Resource::*},
    query::Query,
};

#[derive(PartialEq, sqlx::Type, Deserialize)]
#[sqlx(type_name = "MEDIA_TYPE", rename_all = "lowercase")]
enum MediaType {
    Image,
    Video,
}

#[derive(askama_axum::Template)]
#[template(path = "posts.html")]
struct PostsTemplate {
    signed_in: bool,
    results: Vec<QueriedPosts>,
}

#[derive(sqlx::FromRow)]
struct QueriedPosts {
    url: String,
    thumbnail_path: String,
}

#[derive(askama_axum::Template)]
#[template(path = "post.html")]
struct PostTemplate {
    signed_in: bool,
    post: PostInformation,
    tags: Vec<PostTag>,
    pools: Vec<(i32, String)>,
    posted_at: String,
    posted_ago: String,
    file_size: String,
}

#[derive(askama_axum::Template)]
#[template(path = "upload.html")]
struct UploadTemplate {
    signed_in: bool,
}

#[derive(Serialize)]
struct UploadResponse {
    post_id: i32,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "POST_VOTE", rename_all = "lowercase")]
enum PostVote {
    Like,
    Dislike,
}

#[derive(sqlx::FromRow)]
struct PostInformation {
    pub id: i32,
    pub uploader_id: Option<Uuid>,
    pub md5: String,
    pub width: i32,
    pub height: i32,
    pub source: String,
    pub uploaded_at: time::OffsetDateTime,
    pub media_type: MediaType,
    pub file_size: i64,
    pub media_path: String,
    /* Additional information */
    pub uploader: String,
    pub group_colour: Option<String>,
    pub score: i64,
    pub user_vote: Option<PostVote>,
    pub user_favourited: bool,
}

#[derive(sqlx::FromRow)]
struct PostTag {
    pub name: String,
    pub colour: String,
    pub count: i32,
    pub rank: i32,
}

async fn get_post_info(db: &sqlx::PgPool, id: i32) -> crate::Result<(PostInformation, Vec<PostTag>, Vec<(i32, String)>)> {
    let pi: PostInformation = sqlx::query_as("
        WITH post_info AS (
            SELECT * FROM posts WHERE id = $1
        ),
        user_info AS (
            SELECT users.username AS uploader, users.id, users.group_id
            FROM post_info
            LEFT JOIN users ON users.id = post_info.uploader_id
        ),
        group_info AS (
            SELECT groups.id, groups.colour AS group_colour
            FROM user_info
            LEFT JOIN groups ON groups.id = user_info.group_id
        ),
        user_vote AS (
            SELECT post_id, user_votes.vote AS user_vote
            FROM user_votes
            LEFT JOIN users ON users.id = user_votes.user_id
            WHERE user_votes.post_id = $1
        ), post_score AS (
            SELECT user_votes.post_id,
                SUM(CASE WHEN user_votes.vote = 'like' THEN 1 ELSE 0 END) -
                SUM(CASE WHEN user_votes.vote = 'dislike' THEN 1 ELSE 0 END) AS score
            FROM user_votes
            LEFT JOIN posts ON posts.id = user_votes.post_id 
            WHERE user_votes.post_id = $1
            GROUP BY user_votes.post_id
        ),
        favourite AS (
            SELECT true AS favourited
            FROM user_favourites
            WHERE post_id = $1
        )
        SELECT
            post_info.id,
            post_info.uploader_id,
            post_info.md5,
            post_info.width,
            post_info.height,
            post_info.source,
            post_info.uploaded_at,
            post_info.media_type,
            post_info.file_size,
            ('/static/media/' || post_info.media_path) AS media_path,

            COALESCE(user_info.uploader, 'Anonymous') AS uploader,
            COALESCE(post_score.score, 0) AS score,
            group_info.group_colour,
            user_vote.user_vote,
            COALESCE(favourite.favourited, false) AS user_favourited
        FROM post_info
        LEFT JOIN user_info ON post_info.uploader_id = user_info.id
        LEFT JOIN group_info ON user_info.group_id = group_info.id
        LEFT JOIN post_score ON $1 = post_score.post_id
        LEFT JOIN user_vote ON post_info.id = user_vote.post_id
        LEFT JOIN favourite ON post_info.id = $1;
    ")  .bind(id)
        .fetch_one(db)
        .await?;

    let tags: Vec<PostTag> = sqlx::query_as("
        SELECT t.name AS name, c.colour AS colour, COUNT(pt_all.post_id) AS count, c.rank AS rank
        FROM post_tags pt
        JOIN tags t
            ON pt.tag_id = t.id
        JOIN tag_categories c ON t.category = c.id
        LEFT JOIN post_tags pt_all ON t.id = pt_all.tag_id
        WHERE pt.post_id = $1
        GROUP BY t.id, c.colour, c.rank
        ORDER BY name;
    ")  .bind(id)
        .fetch_all(db)
        .await?;

    let pool: Vec<(i32, String)> = sqlx::query_scalar("
        SELECT p.id, p.name
        FROM post_pools pp
        JOIN pools p ON pp.pool_id = p.id
        WHERE pp.post_id = $1
    ")  .bind(id)
        .fetch_all(db)
        .await?;

    Ok((pi, tags, pool))
}

pub fn routes() -> Router<crate::State> {
    Router::new()
        .route("/posts", get(posts))
        .route("/posts/:id", get(post_page))
        .route("/posts/upload", get(upload))

        .route("/api/posts/upload", post(api_upload))
}

async fn posts(
    auth: Authentication,
    State(state): State<crate::State>,
    query: Query,
) -> crate::Result<impl IntoResponse> {
    let results: Vec<QueriedPosts> = sqlx::query_as("
        SELECT ('/posts/' || id)                    AS url,
               ('/static/thumb/' || thumbnail_path) AS thumbnail_path
        FROM posts
    ")  .fetch_all(&state.db)
        .await?;

    log::debug!("Serving query {query:?}");

    Ok(PostsTemplate {
        signed_in: auth.signed_in(),
        results,
    })
}

async fn post_page(
    auth: Authentication,
    State(state): State<crate::State>,
    extract::Path(id): extract::Path<i32>,
) -> crate::Result<impl IntoResponse> {
    let (post, tags, pools) = get_post_info(&state.db, id).await?;

    let posted_at = post.uploaded_at
        .format(&time::format_description::well_known::Rfc2822)
        .expect("Couldn't convert post timestamp to RFC2822 string");
    let posted_ago = timeago::Formatter::new().convert((time::OffsetDateTime::now_utc() - post.uploaded_at).unsigned_abs());
    let file_size = crate::readable_file_size(post.file_size as _)?;

    Ok(PostTemplate {
        signed_in: auth.signed_in(),
        post,
        tags,
        pools,
        file_size,
        posted_at,
        posted_ago,
    })
}

async fn upload(auth: Authentication) -> impl IntoResponse {
    UploadTemplate { signed_in: auth.signed_in(), }
}

async fn api_upload(
    user: Authentication,
    State(state): State<crate::State>,
    mut multipart: Multipart,
) -> crate::Result<Json<Vec<UploadResponse>>> {
    if !user.has(Permission(Create, Posts)).await? {
        return Err(crate::Error::Unauthorized);
    }

    let mut res = Vec::new();
    // For each file
    while let Some(field) = multipart.next_field().await? {
        res.push(save_multipart_file(&user, &state, field).await?);
    }

    Ok(Json(res))
}

async fn save_multipart_file(
    user: &Authentication,
    state: &crate::State,
    mut field: Field<'_>,
) -> crate::Result<UploadResponse> {
    // Determine whether the file type is supported
    let first_chunk = field.chunk().await?.ok_or_else(|| crate::Error::BadRequest("empty file".to_string()))?;
    let mime = infer::get(&first_chunk)
        .ok_or(crate::Error::UnsupportedMediaType(String::from("couldn't determine")))?;
    let ext = mime.extension();
    if !crate::UNDERSTOOD_MIMES.contains(&mime.mime_type()) {
        return Err(crate::Error::UnsupportedMediaType(format!("MIME type {mime} is unsupported")));
    }
    log::debug!("Inferred from {}b: {mime}", first_chunk.len());

    // Write it to a temporary path while calculating its hash
    let temp_path = std::env::temp_dir().join(temp_filename());
    let mut temp_file = create_open(&temp_path).await?;
    let hash = {
        let mut hasher = md5::Md5::new();

        hasher.update(&first_chunk);
        temp_file.write_all(&first_chunk).await?;
        while let Some(chunk) = field
            .chunk()
            .await?
        {
            hasher.update(&chunk);
            temp_file.write_all(&chunk).await?;
        }

        hasher.finalize().encode_hex::<String>()
    };
    let hash_tree = PathBuf::from(&hash[0..2]).join(&hash[2..4]);
    let hash_path = hash_tree.join(&hash).with_extension(ext);

    // Create a database entry for it
    let (w, h) = media_dimensions(&temp_path)?;
    let media_type = match mime.matcher_type() {
        MatcherType::Image => MediaType::Image,
        MatcherType::Video => MediaType::Video,
        _ => unreachable!(),
    };
    let file_size: i64 = fs::metadata(&temp_path)
        .await?
        .size()
        .try_into()
        .map_err(|_| crate::Error::ContentTooLarge(String::from("Files over 9,300 petabytes are not supported")))?;
    let thumb_hash_path = hash_tree.join(&hash).with_extension("webp");
    let thumb_path = state.config.data.thumbnails().join(&thumb_hash_path);

    let post_id = sqlx::query_scalar("
        INSERT INTO posts (uploader_id, md5, width, height, media_type, file_size, media_path, thumbnail_path)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
    ")  .bind(user.id)
        .bind(&hash)
        .bind(w)
        .bind(h)
        .bind(media_type)
        .bind(file_size)
        .bind(hash_path.to_string_lossy())
        .bind(thumb_hash_path.to_string_lossy())
        .fetch_one(&state.db)
        .await;

    // Move the media to its resting place, report the upload as a duplicate, or otherwise fail
    match post_id {
        Ok(post_id) => {
            let resting_place = state.config.data.media().join(&hash_path);
            fs::create_dir_all(resting_place.parent().unwrap()).await?;
            fs::rename(temp_path, &resting_place).await?;
        
            create_thumbnail(&resting_place, &thumb_path, state.config.data.thumbnails.resolution).await?;
            
            Ok(UploadResponse { post_id, })
        },
        Err(sqlx::Error::Database(dbe)) if dbe.constraint() == Some("posts_md5_key") => {
            fs::remove_file(&temp_path).await?;
            Err(crate::Error::Conflict(String::from("Duplicate post")))
        },
        Err(e) => Err(e.into()),
    }
}

/// TODO: It might be worth reusing the ictx/input/decoder from this function
/// in create_thumbnail etc. Not sure of the performance implication of
/// parsing the file twice for this.
fn media_dimensions<P: AsRef<Path>>(path: P) -> crate::Result<(i32, i32)> {
    let ictx = ffmpeg::format::input(&path)?;
    let input = ictx.streams().best(ffmpeg::media::Type::Video)
        .ok_or(crate::Error::BadRequest("Media has no streams".to_string()))?;
    let context_decoder = CodecContext::from_parameters(input.parameters())?;
    let decoder = context_decoder.decoder().video()?;

    Ok((decoder.width() as i32, decoder.height() as i32))
}

// Create and open a file as read+write, creating its subdirectories if they
// don't already exist.
async fn create_open<P: AsRef<Path>>(path: P) -> crate::Result<File> {
    let dirs = path.as_ref().parent().expect("create_open called with empty path");
    fs::create_dir_all(&dirs).await?;

    Ok(File::create_new(path).await?)
}

async fn create_thumbnail(src: &Path, dst: &Path, res: u32) -> crate::Result<()> {
    let original_frame = first_frame(&src).context("Couldn't extract first frame of uploaded media")?;
    let scaled_frame = scale_frame(original_frame, res)?;
    write_frame(scaled_frame, dst).await.context("Couldn't write out computed frame")?;

    Ok(())
}

/// Returns the first full frame from the input.
fn first_frame(src: &Path) -> crate::Result<Video> {
    let mut ictx = ffmpeg::format::input(&src)?;
    let seek_sec = (ictx.duration() / 2).rescale((1, 1), ffmpeg::rescale::TIME_BASE);
    ictx.seek(seek_sec, ..seek_sec)?;

    let input = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or(crate::Error::BadRequest("Media has no streams".to_string()))?;
    let video_stream_index = input.index();

    let context_decoder = CodecContext::from_parameters(input.parameters())?;
    let mut decoder = context_decoder.decoder().video()?;

    // I'm not sure if there's an opportunity for a DOS by sending a large video
    // that contains no full frames so I'll play it safe.
    let mut frames_decoded = 0;
    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            let mut frame = Video::empty();
            while decoder.receive_frame(&mut frame).is_err() {
                decoder.send_packet(&packet)?;
                if frames_decoded > 10 {
                    return Err(crate::Error::UnsupportedMediaType(
                        "Couldn't decode a thumbnail frame".to_string()
                    ))
                }

                frames_decoded += 1;
            };

            return Ok(frame);
        }
    }

    Err(crate::Error::UnsupportedMediaType("couldn't decode a frame".to_string()))
}

fn scale_frame(frame: Video, to: u32) -> crate::Result<Video> {
    let (sw, sh) = (frame.width(), frame.height());
    let ratio = (to as f32 / sw as f32).min(to as f32 / sh as f32);
    let (w, h) = ((sw as f32 * ratio) as u32, (sh as f32 * ratio) as u32);

    let mut scaler = ScalingContext::get(
        frame.format(),
        sw, sh,
        ffmpeg::format::Pixel::YUV420P,
        w, h,
        ffmpeg::software::scaling::flag::Flags::BILINEAR,
    )?;

    let mut scaled_frame = Video::empty();
    scaler.run(&frame, &mut scaled_frame)?;
    scaled_frame.set_pts(Some(0));

    Ok(scaled_frame)
}

// TODO: use avif instead of webp
async fn write_frame(frame: Video, dst: &Path) -> crate::Result<()> {
    // HACK: Make sure the directory exists
    fs::create_dir_all(dst.parent().expect("write_frame called with invalid dst")).await?;

    let webp_encoder = codec::encoder::find(codec::id::Id::WEBP)
        .expect("ffmpeg couldn't find thumbnail codec encoder");
    let codec_ctx = CodecContext::new_with_codec(webp_encoder);
    let mut encoder = codec_ctx.encoder().video()?;

    encoder.set_height(frame.height());
    encoder.set_width(frame.width());
    encoder.set_format(frame.format());
    encoder.set_time_base(ffmpeg::Rational::new(1, 1));
    let mut opened_encoder = encoder.open()?;

    let mut output = ffmpeg::format::output(dst)?;
    let mut output_stream = output.add_stream(webp_encoder)?;
    output_stream.set_parameters(&opened_encoder);

    output.write_header()?;
    opened_encoder.send_frame(&frame).context("Couldn't send decoded frame to thumbnail encoder")?;
    opened_encoder.send_eof()?;

    let mut encoded = ffmpeg::Packet::empty();
    while opened_encoder.receive_packet(&mut encoded).is_ok() {
        encoded.write(&mut output)?;
    }

    output.write_trailer()?;

    Ok(())
}

fn temp_filename() -> String {
    let mut out = [0u8; 16];
    rand_core::OsRng.fill_bytes(&mut out);
    hex::encode(out)
}
