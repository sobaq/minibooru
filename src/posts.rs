use std::{os::unix::fs::MetadataExt, path::{Path, PathBuf}};
use anyhow::Context;
use argon2::password_hash::rand_core::{self, RngCore};
use ffmpeg::{codec::{self, context::Context as CodecContext}, frame::Video, software::scaling::context::Context as ScalingContext, Rescale};
use askama_axum::IntoResponse;
use axum::{extract::{self, multipart::Field, Multipart, State}, routing::{get, post}, Json, Router};
use hex::ToHex;
use md5::Digest;
use serde::Serialize;
use tokio::{fs::{self, File}, io::AsyncWriteExt};
use infer::MatcherType;

use crate::{auth::{Authentication, Operation::*, Permission, Resource::*}, error::ResultExt};

#[derive(askama_axum::Template)]
#[template(path = "posts.html")]
struct Posts {
    signed_in: bool,
    results: Vec<Result>,
}

#[derive(sqlx::FromRow)]
struct Result {
    url: String,
    thumbnail_path: String,
}

#[derive(askama_axum::Template)]
#[template(path = "post.html")]
struct Post {
    signed_in: bool,
    media_url: String,
}

#[derive(askama_axum::Template)]
#[template(path = "upload.html")]
struct Upload {
    signed_in: bool,
}

#[derive(Serialize)]
struct UploadResponse {
    post_id: i32,
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
) -> crate::Result<impl IntoResponse> {
    let results: Vec<Result> = sqlx::query_as("
        SELECT ('/posts/' || id)                    AS url,
               ('/static/thumb/' || thumbnail_path) AS thumbnail_path
        FROM posts
    ")  .fetch_all(&state.db)
        .await?;

    Ok(Posts {
        signed_in: auth.signed_in(),
        results,
    })
}

async fn post_page(
    auth: Authentication,
    State(state): State<crate::State>,
    extract::Path(id): extract::Path<i32>,
) -> crate::Result<impl IntoResponse> {
    let media_url: String = sqlx::query_scalar("
        SELECT '/static/media/' || media_path
        FROM posts
        WHERE id = $1
    ")  .bind(id)
        .fetch_one(&state.db)
        .await?;

    Ok(Post { signed_in: auth.signed_in(), media_url })
}

async fn upload(auth: Authentication) -> impl IntoResponse {
    Upload { signed_in: auth.signed_in(), }
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

/// TODO:
///   - ensure we've read a fair amount (say 4k) of an in file to ensure we can
///     figure its type out accurately.
///   - check for duplicates earlier, maybe a dedicated endpoint the client can
///     request before beginning upload
///   - (!!) ensure temporary file is deleted if upload fails
async fn save_multipart_file(
    user: &Authentication,
    state: &crate::State,
    mut field: Field<'_>,
) -> crate::Result<UploadResponse> {
    // Determine whether the file type is supported
    let first_chunk = field.chunk().await?.ok_or_else(|| crate::Error::BadRequest("empty file".to_string()))?;
    let mime = infer::get(&first_chunk)
        .ok_or(crate::Error::UnsupportedMediaType(String::from("couldn't determine")))?;
    if !crate::UNDERSTOOD_MIMES.contains(&mime.mime_type()) {
        return Err(crate::Error::UnsupportedMediaType(format!("MIME type {mime} is unsupported")));
    }
    let ext = mime.extension();

    log::debug!("Inferred from {}b: {mime}", first_chunk.len());

    // Save it to a temporary path
    let temp_path = state.config.data.temp().join(temp_filename());
    let mut temp_file = create_open(&temp_path).await?;
    let hash = {
        let mut hasher = md5::Md5::new();

        // Remember this is the SECOND part of the file.
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

    // Save a thumbnail
    let thumb_hash_path = hash_tree.join(&hash).with_extension("webp");
    let thumb_path = state.config.data.thumbnails().join(&thumb_hash_path);
    let (w, h) = create_thumbnail(&temp_path, &thumb_path).await?;

    let resting_place = state.config.data.media().join(&hash_path);
    fs::create_dir_all(resting_place.parent().unwrap()).await?;
    fs::rename(&temp_path, &resting_place).await?;

    // Create post in database
    let media_type = match mime.matcher_type() {
        MatcherType::Image => "image",
        MatcherType::Video => "video",
        _ => unreachable!(),
    };

    let file_size: i64 = fs::metadata(resting_place)
        .await?
        .size()
        .try_into()
        .map_err(|_| crate::Error::ContentTooLarge(String::from("wtf")))?;

    let post_id: i32 = sqlx::query_scalar("
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
        .await
        .on_constraint("posts_md5_key", |_| crate::Error::BadRequest(String::from("Duplicate post")))?;

    Ok(UploadResponse { post_id, })
}

// Create and open a file as read+write, creating its subdirectories if they
// don't already exist.
async fn create_open<P: AsRef<Path>>(path: P) -> crate::Result<File> {
    let dirs = path.as_ref().parent().expect("create_open called with empty path");
    fs::create_dir_all(&dirs).await?;

    Ok(File::create_new(path).await?)
}

/// TODO: This function returns the source video's width and height for the database
/// because it's the only place with an open video context. Bit of a hack.
async fn create_thumbnail(src: &Path, dst: &Path) -> crate::Result<(i32, i32)> {
    let (frame, w, h) = first_frame(&src).context("Couldn't extract first frame of uploaded media")?;
    write_frame(frame, dst).await.context("Couldn't write out computed frame")?;

    Ok((w, h))
}

// TODO: use avif instead of webp
async fn write_frame(frame: Video, dst: &Path) -> crate::Result<()> {
    // HACK: Make sure the directory exists
    fs::create_dir_all(dst.parent().expect("write_frame called with invalid dst")).await?;

    // let avif_encoder = codec::encoder::find(codec::id::Id::AV1)
    let webp_encoder = codec::encoder::find(codec::id::Id::WEBP)
        .expect("ffmpeg couldn't find thumbnail codec encoder");
    let codec_ctx = CodecContext::new_with_codec(webp_encoder);
    let mut encoder = codec_ctx.encoder().video()?;

    encoder.set_height(frame.height());
    encoder.set_width(frame.width());
    encoder.set_format(frame.format());
    encoder.set_time_base(ffmpeg::Rational::new(1, 1));
    // unsafe { ffmpeg::ffi::av_opt_set((*encoder.as_mut_ptr()).priv_data, c"crf".as_ptr(), c"28".as_ptr(), 0); }
    // encoder.set_aspect_ratio(frame.aspect_ratio());
    // encoder.set_frame_rate::<f64>(None);
    // encoder.set_flags(codec::Flags::GLOBAL_HEADER);
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

/// Returns the first full image/frame from an input
fn first_frame(src: &Path) -> crate::Result<(Video, i32, i32)> {
    let mut ictx = ffmpeg::format::input(&src)?;
    // move a few seconds forward to hopefully get a good thumbnail
    // TODO: extract thumbnail from middle of video, e.g. actually determine its length
    let seek_sec = 3_i64.rescale((1, 1), ffmpeg::rescale::TIME_BASE);
    ictx.seek(seek_sec, ..seek_sec)?;

    let input = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or(crate::Error::BadRequest("Media has no streams".to_string()))?;
    let video_stream_index = input.index();

    let context_decoder = CodecContext::from_parameters(input.parameters())?;
    let mut decoder = context_decoder.decoder().video()?;
    let (w, h) = (decoder.width() as i32, decoder.height() as i32);

    let mut scaler = ScalingContext::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::YUV420P,
        decoder.width(),
        decoder.height(),
        ffmpeg::software::scaling::flag::Flags::BILINEAR,
    )?;

    // I'm not sure if there's an opportunity for a DOS by sending a large video
    // that contains no full frames so I'll play it safe.
    let mut frames_decoded = 0;
    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            let mut decoded_video = Video::empty();
            while decoder.receive_frame(&mut decoded_video).is_err() {
                decoder.send_packet(&packet)?;
                if frames_decoded > 10 {
                    return Err(crate::Error::UnsupportedMediaType(
                        "Couldn't decode a thumbnail frame".to_string()
                    ))
                }
                frames_decoded += 1;
            };

            let mut frame = Video::empty();
            scaler.run(&decoded_video, &mut frame)?;

            decoder.send_eof()?;
            frame.set_pts(Some(0));
            return Ok((frame, w, h));
        }
    };

    decoder.send_eof()?;
    Err(crate::Error::UnsupportedMediaType("couldn't decode a frame".to_string()))
}

fn temp_filename() -> String {
    let mut out = [0u8; 16];
    rand_core::OsRng.fill_bytes(&mut out);
    hex::encode(out)
}