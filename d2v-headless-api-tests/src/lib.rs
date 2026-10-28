use std::env;
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

use printpdf::*;
use serde::Deserialize;

pub struct Config {
    pub base_url: String,
    pub secret: String,
    pub project_root: String,
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
        Config {
            base_url,
            secret,
            project_root,
        }
    }

    pub fn job_output_dir(&self, job_id: &str) -> String {
        format!("{}/output/{job_id}", self.project_root)
    }

    pub fn job_temp_dir(&self, job_id: &str) -> String {
        format!("{}/temp/{job_id}", self.project_root)
    }
}

#[derive(Debug, Deserialize)]
pub struct StartJobResponse {
    #[serde(rename = "jobId")]
    pub job_id: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    ProcessingOrNotFound,
    Done,
    Failed(String),
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

pub fn poll_status(config: &Config, job_id: &str, timeout: Duration) -> JobStatus {
    let http_client = reqwest::blocking::Client::new();
    let poll_started_at = Instant::now();

    sleep(Duration::from_secs(5));

    loop {
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

        match status_response.status.as_str() {
            "done" => return JobStatus::Done,
            "failed" => return JobStatus::Failed(status_response.message.unwrap_or_default()),
            _ => {}
        }

        if poll_started_at.elapsed() > timeout {
            panic!("job {job_id} did not reach done/failed within {timeout:?}");
        }

        sleep(Duration::from_secs(2));
    }
}

pub struct FixtureSection {
    pub name: &'static str,
    pub lines: Vec<&'static str>,
}

pub fn fixture_sections() -> Vec<FixtureSection> {
    vec![
        FixtureSection {
            name: "plain_paragraph",
            lines: vec![
                "The quick brown fox jumps over the lazy dog.",
                "This page contains a single plain paragraph with no special formatting.",
            ],
        },
        FixtureSection {
            name: "table_like_content",
            lines: vec![
                "Product Catalog",
                "Name: Widget A. Price: 10 dollars. Quantity: 5 units.",
                "Name: Widget B. Price: 20 dollars. Quantity: 3 units.",
                "Name: Widget C. Price: 15 dollars. Quantity: 8 units.",
            ],
        },
        FixtureSection {
            name: "special_characters",
            lines: vec![
                "Special characters test page.",
                "Accented letters: cafe, resume, naive, senor.",
                "Currency and punctuation: 99.99 dollars, 50 pounds, 20 euros - a dash and an ellipsis...",
            ],
        },
    ]
}

pub fn generate_test_pdf() -> Vec<u8> {
    let mut pdf_document = PdfDocument::new("D2V Headless API Test Fixture");
    let mut pdf_pages = Vec::new();

    for fixture_section in fixture_sections() {
        let mut page_operations = vec![
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point::new(Mm(20.0), Mm(270.0)),
            },
            Op::SetFont {
                font: PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                size: Pt(14.0),
            },
            Op::SetLineHeight { lh: Pt(18.0) },
            Op::SetFillColor {
                col: Color::Rgb(Rgb {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    icc_profile: None,
                }),
            },
        ];

        let line_count = fixture_section.lines.len();
        for (line_index, line_text) in fixture_section.lines.into_iter().enumerate() {
            page_operations.push(Op::ShowText {
                items: vec![TextItem::Text(line_text.to_string())],
            });
            if line_index + 1 < line_count {
                page_operations.push(Op::AddLineBreak);
            }
        }
        page_operations.push(Op::EndTextSection);

        pdf_pages.push(PdfPage::new(Mm(210.0), Mm(297.0), page_operations));
    }

    pdf_document
        .with_pages(pdf_pages)
        .save(&PdfSaveOptions::default(), &mut Vec::new())
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
