use std::env;
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

use printpdf::*;
use serde::Deserialize;

pub struct TemporarilyMovedPath {
    original_path: String,
    backup_path: String,
}

impl TemporarilyMovedPath {
    pub fn move_aside_if_present(path: &str) -> Option<Self> {
        if !std::path::Path::new(path).exists() {
            return None;
        }
        let backup_path = format!("{path}.d2v-test-backup");
        std::fs::rename(path, &backup_path)
            .unwrap_or_else(|e| panic!("failed to move {path} aside: {e}"));
        Some(Self {
            original_path: path.to_string(),
            backup_path,
        })
    }
}

impl TemporarilyMovedPath {
    pub fn discard_backup(self) {
        let _ = std::fs::remove_file(&self.backup_path);
        std::mem::forget(self);
    }
}

impl Drop for TemporarilyMovedPath {
    fn drop(&mut self) {
        let _ = std::fs::rename(&self.backup_path, &self.original_path);
    }
}

pub fn move_every_entry_aside(directory_path: &str) -> Vec<TemporarilyMovedPath> {
    move_entries_aside_except(directory_path, |_| false)
}

pub fn move_entries_aside_except(
    directory_path: &str,
    keep_entry_named: impl Fn(&str) -> bool,
) -> Vec<TemporarilyMovedPath> {
    let Ok(directory_entries) = std::fs::read_dir(directory_path) else {
        return Vec::new();
    };

    directory_entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| !keep_entry_named(&entry.file_name().to_string_lossy()))
        .filter_map(|entry| {
            TemporarilyMovedPath::move_aside_if_present(&entry.path().to_string_lossy())
        })
        .collect()
}

pub fn discard_all_backups(guards: Vec<TemporarilyMovedPath>) {
    for guard in guards {
        guard.discard_backup();
    }
}

pub fn move_named_file_aside_in_every_subdirectory(
    parent_directory_path: &str,
    file_name: &str,
) -> Vec<TemporarilyMovedPath> {
    let Ok(directory_entries) = std::fs::read_dir(parent_directory_path) else {
        return Vec::new();
    };

    directory_entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            TemporarilyMovedPath::move_aside_if_present(&format!(
                "{}/{file_name}",
                entry.path().to_string_lossy()
            ))
        })
        .collect()
}

pub struct Config {
    pub base_url: String,
    pub secret: String,
    pub project_root: String,
    pub engine_model: String,
}

impl Config {
    pub fn from_env() -> Self {
        let base_url = env::var("D2V_BASE_URL")
            .expect("D2V_BASE_URL must be set (e.g. http://localhost:5678)");
        let secret = env::var("D2V_WEBHOOK_SECRET")
            .expect("D2V_WEBHOOK_SECRET must be set to the headless API's shared secret");
        let project_root = env::var("D2V_PROJECT_ROOT").expect(
            "D2V_PROJECT_ROOT must be set to the Eggocado-D2V checkout containing output/ and temp/",
        );
        let engine_model = env::var("D2V_ENGINE_MODEL").expect(
            "D2V_ENGINE_MODEL must be set to a transcription engine:model available on this instance (e.g. codex:gpt-5.6-sol, claude:claude-sonnet-5, or ollama:<model>)",
        );
        Config {
            base_url,
            secret,
            project_root,
            engine_model,
        }
    }

    pub fn job_output_dir(&self, job_id: &str) -> String {
        format!("{}/output/{job_id}", self.project_root)
    }

    pub fn job_temp_dir(&self, job_id: &str) -> String {
        format!("{}/temp/{job_id}", self.project_root)
    }

    pub fn temp_dir(&self) -> String {
        format!("{}/temp", self.project_root)
    }
}

#[derive(Debug, Deserialize)]
pub struct StartJobResponse {
    #[serde(rename = "jobId")]
    pub job_id: String,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    NotFound,
    Processing,
    Done,
    Failed(String),
}

fn parse_job_status(status_response: StatusResponse) -> JobStatus {
    match status_response.status.as_str() {
        "not_found" => JobStatus::NotFound,
        "processing" => JobStatus::Processing,
        "done" => JobStatus::Done,
        "failed" => JobStatus::Failed(status_response.message.unwrap_or_default()),
        other => panic!("status endpoint returned unknown status {other:?}"),
    }
}

#[derive(Debug, Deserialize)]
struct StatusResponse {
    status: String,
    message: Option<String>,
}

pub enum Field<'a> {
    Text(&'a str, &'a str),
    File {
        name: &'a str,
        filename: &'a str,
        mime: &'a str,
        bytes: Vec<u8>,
    },
}

pub fn start_job_expecting_bad_request(config: &Config, fields: &[Field]) -> String {
    let http_client = reqwest::blocking::Client::new();
    let mut multipart_form = reqwest::blocking::multipart::Form::new();

    for field in fields {
        multipart_form = match field {
            Field::Text(name, value) => multipart_form.text(name.to_string(), value.to_string()),
            Field::File {
                name,
                filename,
                mime,
                bytes,
            } => {
                let file_part = reqwest::blocking::multipart::Part::bytes(bytes.clone())
                    .file_name(filename.to_string())
                    .mime_str(mime)
                    .expect("invalid mime type");
                multipart_form.part(name.to_string(), file_part)
            }
        };
    }

    let http_response = http_client
        .post(format!("{}/webhook/d2v-headless", config.base_url))
        .header("x-webhook-secret", &config.secret)
        .multipart(multipart_form)
        .send()
        .expect("failed to send start-job request");

    let http_status = http_response.status();
    let response_body_text = http_response
        .text()
        .expect("failed to read start-job response body");
    assert_eq!(
        http_status.as_u16(),
        400,
        "expected HTTP 400 Bad Request, got {http_status}: {response_body_text}"
    );

    #[derive(Deserialize)]
    struct ErrorResponse {
        error: String,
    }
    let error_response: ErrorResponse = serde_json::from_str(&response_body_text)
        .unwrap_or_else(|parse_error| {
            panic!("failed to parse bad-request response: {parse_error}\nbody: {response_body_text}")
        });
    error_response.error
}

pub fn start_job(config: &Config, fields: &[Field]) -> StartJobResponse {
    let http_client = reqwest::blocking::Client::new();
    let mut multipart_form = reqwest::blocking::multipart::Form::new();

    for field in fields {
        multipart_form = match field {
            Field::Text(name, value) => multipart_form.text(name.to_string(), value.to_string()),
            Field::File {
                name,
                filename,
                mime,
                bytes,
            } => {
                let file_part = reqwest::blocking::multipart::Part::bytes(bytes.clone())
                    .file_name(filename.to_string())
                    .mime_str(mime)
                    .expect("invalid mime type");
                multipart_form.part(name.to_string(), file_part)
            }
        };
    }

    let http_response = http_client
        .post(format!("{}/webhook/d2v-headless", config.base_url))
        .header("x-webhook-secret", &config.secret)
        .multipart(multipart_form)
        .send()
        .expect("failed to send start-job request");

    let http_status = http_response.status();
    let response_body_text = http_response
        .text()
        .expect("failed to read start-job response body");
    assert!(
        http_status.is_success(),
        "start_job got HTTP {http_status}: {response_body_text}"
    );

    serde_json::from_str(&response_body_text).unwrap_or_else(|parse_error| {
        panic!("failed to parse start-job response: {parse_error}\nbody: {response_body_text}")
    })
}

pub fn check_status_once(config: &Config, job_id: &str) -> JobStatus {
    let http_client = reqwest::blocking::Client::new();

    let http_response = http_client
        .get(format!("{}/webhook/d2v-headless-status", config.base_url))
        .query(&[("jobId", job_id)])
        .header("x-webhook-secret", &config.secret)
        .send()
        .expect("failed to send status request");

    let response_body_text = http_response
        .text()
        .expect("failed to read status response body");
    let status_response: StatusResponse = serde_json::from_str(&response_body_text)
        .unwrap_or_else(|parse_error| {
            panic!("failed to parse status response: {parse_error}\nbody: {response_body_text}")
        });

    parse_job_status(status_response)
}

pub fn poll_status(config: &Config, job_id: &str, timeout: Duration) -> JobStatus {
    let poll_started_at = Instant::now();

    sleep(Duration::from_secs(5));

    loop {
        match check_status_once(config, job_id) {
            JobStatus::Done => return JobStatus::Done,
            JobStatus::Failed(message) => return JobStatus::Failed(message),
            JobStatus::NotFound | JobStatus::Processing => {}
        }

        if poll_started_at.elapsed() > timeout {
            panic!("job {job_id} did not reach done/failed within {timeout:?}");
        }

        sleep(Duration::from_secs(2));
    }
}

const FIXTURE_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
<style>
body { font-family: Helvetica, sans-serif; font-size: 14px; }
.page + .page { page-break-before: always; }
table { border-collapse: collapse; margin-top: 10px; }
th, td { border: 1px solid #000000; padding: 4px 10px; text-align: left; }
img { width: 80mm; height: auto; }
</style>
</head>
<body>
<div class="page">
<p>The quick brown fox jumps over the lazy dog.</p>
<p>This page contains a single plain paragraph with no special formatting.</p>
</div>
<div class="page">
<p>Product Catalog</p>
<table>
<tr><th>Name</th><th>Price</th><th>Quantity</th></tr>
<tr><td>Widget A</td><td>10 dollars</td><td>5 units</td></tr>
<tr><td>Widget B</td><td>20 dollars</td><td>3 units</td></tr>
<tr><td>Widget C</td><td>15 dollars</td><td>8 units</td></tr>
</table>
</div>
<div class="page">
<p>Special characters test page.</p>
<p>Accented letters: café, résumé, naïve, señor.</p>
<p>Currency and punctuation: $99.99, £50, €20 — a dash and an ellipsis…</p>
<p>Non-Latin scripts: Ελληνικά (Greek), Русский (Russian).</p>
</div>
<div class="page">
<p>Slashes, dates, and URLs test page.</p>
<p>File path: /usr/local/share/d2v/output</p>
<p>Date: 10/28/2026</p>
<p>Measurement: 1/2 cup</p>
<p>Link: https://example.com/docs?query=value&amp;page=2</p>
</div>
<div class="page">
<p>Embedded image test page.</p>
<img src="fixture.png" />
<p>A synthesized three-band color image, embedded above.</p>
</div>
</body>
</html>"#;

fn generate_test_image_png_bytes() -> Vec<u8> {
    let image_width = 240u32;
    let image_height = 160u32;
    let three_band_image = ::image::RgbImage::from_fn(image_width, image_height, |x, _y| {
        if x < image_width / 3 {
            ::image::Rgb([200, 60, 60])
        } else if x < 2 * image_width / 3 {
            ::image::Rgb([60, 160, 80])
        } else {
            ::image::Rgb([60, 90, 200])
        }
    });

    let mut png_bytes = Vec::new();
    three_band_image
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), ::image::ImageFormat::Png)
        .expect("failed to encode synthesized test image as PNG");
    png_bytes
}

pub fn generate_test_pdf() -> Vec<u8> {
    let mut embedded_images = std::collections::BTreeMap::new();
    embedded_images.insert(
        "fixture.png".to_string(),
        Base64OrRaw::Raw(generate_test_image_png_bytes()),
    );
    let embedded_fonts = std::collections::BTreeMap::new();

    let options = GeneratePdfOptions {
        page_width: Some(210.0),
        page_height: Some(297.0),
        margin_top: Some(20.0),
        margin_right: Some(20.0),
        margin_bottom: Some(20.0),
        margin_left: Some(20.0),
        ..Default::default()
    };

    let mut warnings = Vec::new();
    let pdf_document =
        PdfDocument::from_html(FIXTURE_HTML, &embedded_images, &embedded_fonts, &options, &mut warnings)
            .unwrap_or_else(|e| panic!("failed to generate fixture PDF from HTML: {e}"));

    pdf_document.save(&PdfSaveOptions::default(), &mut Vec::new())
}

pub fn wait_for_path_to_exist(path: &str, timeout: Duration) {
    let wait_started_at = Instant::now();

    while !std::path::Path::new(path).exists() {
        if wait_started_at.elapsed() > timeout {
            panic!("{path} did not appear within {timeout:?}");
        }
        sleep(Duration::from_secs(2));
    }
}

pub fn wait_for_file_size_to_stabilize(file_path: &str, timeout: Duration) {
    let wait_started_at = Instant::now();
    let mut last_seen_file_size = 0;
    let mut stable_reading_count = 0;

    loop {
        let current_file_size = std::fs::metadata(file_path)
            .unwrap_or_else(|e| panic!("failed to stat {file_path}: {e}"))
            .len();

        if current_file_size == last_seen_file_size && current_file_size > 0 {
            stable_reading_count += 1;
            if stable_reading_count >= 2 {
                return;
            }
        } else {
            stable_reading_count = 0;
        }
        last_seen_file_size = current_file_size;

        if wait_started_at.elapsed() > timeout {
            panic!("{file_path} did not stop growing within {timeout:?}");
        }

        sleep(Duration::from_millis(500));
    }
}

pub fn ffprobe(video_file_path: &str) -> serde_json::Value {
    let ffprobe_output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-of",
            "json",
            video_file_path,
        ])
        .output()
        .expect("failed to run ffprobe");

    assert!(
        ffprobe_output.status.success(),
        "ffprobe failed on {video_file_path}: {}",
        String::from_utf8_lossy(&ffprobe_output.stderr)
    );

    serde_json::from_slice(&ffprobe_output.stdout).expect("ffprobe produced invalid JSON")
}

pub fn assert_video_has_audio_and_video_streams(video_file_path: &str) {
    let ffprobe_json = ffprobe(video_file_path);
    let media_streams = ffprobe_json["streams"]
        .as_array()
        .expect("ffprobe output had no streams array");
    assert!(
        media_streams.iter().any(|stream| stream["codec_type"] == "video"),
        "{video_file_path} has no video stream"
    );
    assert!(
        media_streams.iter().any(|stream| stream["codec_type"] == "audio"),
        "{video_file_path} has no audio stream"
    );
}

pub fn assert_video_duration_within_range(
    video_file_path: &str,
    min_seconds: f64,
    max_seconds: f64,
) {
    let ffprobe_json = ffprobe(video_file_path);
    let video_duration_seconds: f64 = ffprobe_json["format"]["duration"]
        .as_str()
        .expect("ffprobe format.duration missing")
        .parse()
        .expect("ffprobe format.duration not a number");
    assert!(
        video_duration_seconds >= min_seconds && video_duration_seconds <= max_seconds,
        "{video_file_path} duration {video_duration_seconds}s outside expected range [{min_seconds}, {max_seconds}]"
    );
}

pub fn assert_video_decodes_without_errors(video_file_path: &str) {
    let ffmpeg_decode_output = Command::new("ffmpeg")
        .args(["-v", "error", "-i", video_file_path, "-f", "null", "-"])
        .output()
        .expect("failed to run ffmpeg");
    assert!(
        ffmpeg_decode_output.status.success() && ffmpeg_decode_output.stderr.is_empty(),
        "{video_file_path} failed to decode cleanly: {}",
        String::from_utf8_lossy(&ffmpeg_decode_output.stderr)
    );
}

pub fn assert_srt_is_not_empty(srt_file_contents: &str) {
    assert!(!srt_file_contents.trim().is_empty(), "srt file is empty");
}

pub fn assert_srt_has_well_formed_cues(srt_file_contents: &str) {
    let cue_timestamp_lines: Vec<&str> = srt_file_contents
        .lines()
        .filter(|line| line.contains("-->"))
        .collect();
    assert!(!cue_timestamp_lines.is_empty(), "srt file has no timestamp cues");

    for cue_timestamp_line in cue_timestamp_lines {
        let cue_start_and_end: Vec<&str> = cue_timestamp_line.split("-->").collect();
        assert_eq!(
            cue_start_and_end.len(),
            2,
            "malformed srt timestamp line: {cue_timestamp_line}"
        );
        for timestamp_text in cue_start_and_end {
            let trimmed_timestamp_text = timestamp_text.trim();
            assert!(
                trimmed_timestamp_text.len() == 12
                    && trimmed_timestamp_text.chars().nth(2) == Some(':')
                    && trimmed_timestamp_text.chars().nth(8) == Some(','),
                "srt timestamp not in HH:MM:SS,mmm format: {trimmed_timestamp_text}"
            );
        }
    }
}
