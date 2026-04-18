use axum::{
    body::Body,
    extract::Multipart,
    http::{header, StatusCode},
    response::Response,
    routing::post,
    Router,
};
use std::io::Write;
use tempfile::TempDir;
use tokio::process::Command;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/clip", post(clip))
        .route("/concat", post(concat));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8090").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn clip(mut multipart: Multipart) -> Result<Response<Body>, StatusCode> {
    let tmp = TempDir::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let image_path = tmp.path().join("image.png");
    let audio_path = tmp.path().join("audio.mp3");
    let output_path = tmp.path().join("output.mp4");

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        match name.as_str() {
            "image" => std::fs::write(&image_path, &data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            "audio" => std::fs::write(&audio_path, &data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            _ => {}
        }
    }

    let status = Command::new("ffmpeg")
        .args([
            "-loop", "1",
            "-i", image_path.to_str().unwrap(),
            "-i", audio_path.to_str().unwrap(),
            "-c:v", "libx264", "-tune", "stillimage",
            "-c:a", "aac", "-b:a", "192k",
            "-pix_fmt", "yuv420p",
            "-shortest",
            output_path.to_str().unwrap(),
        ])
        .status()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !status.success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let data = std::fs::read(&output_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "video/mp4")
        .body(Body::from(data))
        .unwrap())
}

async fn concat(mut multipart: Multipart) -> Result<Response<Body>, StatusCode> {
    let tmp = TempDir::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let output_path = tmp.path().join("output.mp4");

    let mut clips: Vec<(String, Vec<u8>)> = Vec::new();
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?.to_vec();
        clips.push((name, data));
    }
    clips.sort_by(|a, b| a.0.cmp(&b.0));

    let filelist_path = tmp.path().join("filelist.txt");
    let mut filelist = std::fs::File::create(&filelist_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for (i, (_, data)) in clips.iter().enumerate() {
        let clip_path = tmp.path().join(format!("clip{:04}.mp4", i));
        std::fs::write(&clip_path, data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        writeln!(filelist, "file '{}'", clip_path.display()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let status = Command::new("ffmpeg")
        .args([
            "-f", "concat",
            "-safe", "0",
            "-i", filelist_path.to_str().unwrap(),
            "-c", "copy",
            output_path.to_str().unwrap(),
        ])
        .status()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !status.success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let data = std::fs::read(&output_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "video/mp4")
        .body(Body::from(data))
        .unwrap())
}
