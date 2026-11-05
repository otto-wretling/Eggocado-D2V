use std::env;
use std::ops::RangeInclusive;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::Once;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use printpdf::*;
use reqwest::StatusCode;
use serde::Deserialize;

pub const CONVERSION_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);
pub const FAILURE_POLL_TIMEOUT: Duration = Duration::from_secs(60);
pub const SERVICE_RELEASE_TIMEOUT: Duration = Duration::from_secs(120);
const FILE_SIZE_STABILIZE_TIMEOUT: Duration = Duration::from_secs(30);
const STATUS_UNREACHABLE_TIMEOUT: Duration = Duration::from_secs(60);
const STATUS_POLL_INITIAL_DELAY: Duration = Duration::from_secs(5);
const STATUS_POLL_INTERVAL: Duration = Duration::from_secs(2);
const CONDITION_POLL_INTERVAL: Duration = Duration::from_secs(2);
const FILE_SIZE_POLL_INTERVAL: Duration = Duration::from_millis(500);
const STABLE_FILE_SIZE_READINGS_REQUIRED: u32 = 2;
const SOCKET_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

pub const DOCUMENT_VIDEO_DURATION_SECONDS: RangeInclusive<f64> = 10.0..=600.0;
pub const SAMPLE_VIDEO_DURATION_SECONDS: RangeInclusive<f64> = 5.0..=3600.0;

const BASE_URL_VARIABLE: &str = "D2V_BASE_URL";
const WEBHOOK_SECRET_VARIABLE: &str = "D2V_WEBHOOK_SECRET";
const PROJECT_ROOT_VARIABLE: &str = "D2V_PROJECT_ROOT";
const ENGINE_MODEL_VARIABLE: &str = "D2V_ENGINE_MODEL";
const OLLAMA_ENGINE_PREFIX: &str = "ollama:";

const PREFLIGHT_PROBE_JOB_ID: &str = "d2v-rust-suite-preflight-probe";
const STATUS_CHECK_NODE: &str = "Check Job State";

const START_JOB_PATH: &str = "/webhook/d2v-headless";
const JOB_STATUS_PATH: &str = "/webhook/d2v-headless-status";
const WEBHOOK_SECRET_HEADER: &str = "x-webhook-secret";
const JOB_ID_QUERY_PARAMETER: &str = "jobId";

const STATUS_NOT_FOUND: &str = "not_found";
const STATUS_PROCESSING: &str = "processing";
const STATUS_DONE: &str = "done";
const STATUS_FAILED: &str = "failed";

const OUTPUT_FOLDER: &str = "output";
const TEMP_FOLDER: &str = "temp";
const STARTED_MARKER_FILE_NAME: &str = ".started";
const STARTED_MARKER_EXECUTION_PREFIX: &str = "execution ";
const FINAL_VIDEO_FILE_NAME: &str = "final_video.mp4";
const FINAL_SUBTITLES_FILE_NAME: &str = "final_video.srt";
const PAGE_FILE_SEPARATOR: &str = "_";
const PAGE_CLIP_FILE_SUFFIX: &str = "_clip.mp4";
const DOCUMENT_META_FILE_NAME: &str = "meta.json";
const PDF_EXTENSION: &str = ".pdf";
const PDF_MIME_TYPE: &str = "application/pdf";
const FALLBACK_JOB_ID: &str = "document";
const SAMPLE_JOB_ID_PREFIX: &str = "sample-";
const JOB_ID_WORD_SEPARATOR: char = '-';
const BACKUP_SUFFIX: &str = ".d2v-test-backup";
const READ_ONLY_DIRECTORY_MODE: u32 = 0o555;
const WRITABLE_DIRECTORY_MODE: u32 = 0o755;

pub mod field {
    pub const MODE: &str = "mode";
    pub const DOCUMENT: &str = "document";
    pub const ENGINE_MODEL: &str = "engineModel";
    pub const VOICE: &str = "voice";
    pub const OVERWRITE: &str = "overwrite";
    pub const ACCELERATION: &str = "acceleration";
    pub const LEFT_PAGE: &str = "leftPage";
    pub const RIGHT_PAGE: &str = "rightPage";
    pub const PAGE_SELECTION: &str = "pageSelection";
    pub const TARGET: &str = "target";
    pub const TEXT_TO_CHANGE: &str = "textToChange";
    pub const SAMPLE_PHRASE: &str = "samplePhrase";
    pub const UPLOADED_DOCUMENT: &str = "data";
}

pub mod mode {
    pub(crate) const NEW: &str = "new";
    pub const CONTINUE: &str = "continue";
    pub(crate) const KNIT: &str = "knit";
    pub(crate) const CHANGE_VOICE: &str = "changeVoice";
    pub const SAMPLE: &str = "sample";
}

pub mod change_voice_target {
    pub const WHOLE_PAGE: &str = "wholePage";
    pub const SELECTED_TEXT: &str = "selectedText";
}

pub mod acceleration {
    pub const CPU: &str = "cpu";
}

pub const DEFAULT_VOICE: &str = "af_heart";
const ALTERNATE_VOICE: &str = "af_bella";
const OVERWRITE_CONFIRMED: &str = "true";
const PAGE_SELECTION_LABEL_PREFIX: &str = "Page ";

pub const SHARED_DOCUMENT_FILENAME: &str = "d2v-rust-suite-new-conversion.pdf";
pub const FIRST_PAGE: u32 = 1;
pub const SECOND_PAGE: u32 = 2;

macro_rules! fixture_image_file_name {
    () => {
        "fixture.png"
    };
}

macro_rules! text_on_second_page {
    () => {
        "Product"
    };
}

const FIXTURE_IMAGE_FILE_NAME: &str = fixture_image_file_name!();
pub const TEXT_ON_SECOND_PAGE: &str = text_on_second_page!();

const FIXTURE_HTML: &str = concat!(
    r#"<!DOCTYPE html>
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
<p>"#,
    text_on_second_page!(),
    r#" Catalog</p>
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
<img src=""#,
    fixture_image_file_name!(),
    r#"" />
<p>A synthesized three-band color image, embedded above.</p>
</div>
</body>
</html>"#
);

const FIXTURE_IMAGE_WIDTH_PIXELS: u32 = 240;
const FIXTURE_IMAGE_HEIGHT_PIXELS: u32 = 160;
const FIXTURE_IMAGE_BAND_COLORS: [[u8; 3]; 3] = [[200, 60, 60], [60, 160, 80], [60, 90, 200]];
const A4_PAGE_WIDTH_MILLIMETERS: f32 = 210.0;
const A4_PAGE_HEIGHT_MILLIMETERS: f32 = 297.0;
const FIXTURE_PAGE_MARGIN_MILLIMETERS: f32 = 20.0;

const FFPROBE_PROGRAM: &str = "ffprobe";
const FFMPEG_PROGRAM: &str = "ffmpeg";
const VIDEO_INSPECTION_PROGRAMS: [&str; 2] = [FFPROBE_PROGRAM, FFMPEG_PROGRAM];
const PROGRAM_VERSION_ARGUMENT: &str = "-version";
const FFPROBE_JSON_REPORT_ARGUMENTS: [&str; 6] =
    ["-v", "error", "-show_format", "-show_streams", "-of", "json"];
const FFMPEG_DECODE_CHECK_ARGUMENTS_BEFORE_INPUT: [&str; 3] = ["-v", "error", "-i"];
const FFMPEG_DECODE_CHECK_ARGUMENTS_AFTER_INPUT: [&str; 3] = ["-f", "null", "-"];
const FFPROBE_STREAMS_KEY: &str = "streams";
const FFPROBE_CODEC_TYPE_KEY: &str = "codec_type";
const FFPROBE_FORMAT_KEY: &str = "format";
const FFPROBE_DURATION_KEY: &str = "duration";
const VIDEO_STREAM_TYPE: &str = "video";
const AUDIO_STREAM_TYPE: &str = "audio";

const SRT_CUE_TIMING_SEPARATOR: &str = "-->";
const SRT_TIMESTAMPS_PER_CUE: usize = 2;
const SRT_TIMESTAMP_LENGTH: usize = "HH:MM:SS,mmm".len();
const SRT_HOURS_MINUTES_SEPARATOR_INDEX: usize = "HH".len();
const SRT_SECONDS_MILLISECONDS_SEPARATOR_INDEX: usize = "HH:MM:SS".len();
const SRT_TIME_SEPARATOR: char = ':';
const SRT_MILLISECONDS_SEPARATOR: char = ',';

pub struct TemporarilyMovedPath {
    original_path: String,
    backup_path: String,
}

impl TemporarilyMovedPath {
    pub fn move_aside_if_present(path: &str) -> Option<Self> {
        if !Path::new(path).exists() {
            return None;
        }
        let backup_path = format!("{path}{BACKUP_SUFFIX}");
        std::fs::rename(path, &backup_path)
            .unwrap_or_else(|e| panic!("failed to move {path} aside: {e}"));
        Some(Self {
            original_path: path.to_string(),
            backup_path,
        })
    }

    fn discard_backup(self) {
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

pub fn is_document_source(entry_name: &str) -> bool {
    entry_name == DOCUMENT_META_FILE_NAME || entry_name.ends_with(PDF_EXTENSION)
}

pub fn is_first_page_or_document_source(entry_name: &str) -> bool {
    entry_name.starts_with(&format!("{FIRST_PAGE}{PAGE_FILE_SEPARATOR}"))
        || is_document_source(entry_name)
}

pub fn page_clip_file_name(page: u32) -> String {
    format!("{page}{PAGE_CLIP_FILE_SUFFIX}")
}

pub struct ReadOnlyDirectory {
    path: String,
}

impl ReadOnlyDirectory {
    pub fn create(path: &str) -> Self {
        std::fs::create_dir_all(path).unwrap_or_else(|e| panic!("failed to create {path}: {e}"));
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(READ_ONLY_DIRECTORY_MODE))
            .unwrap_or_else(|e| panic!("failed to make {path} read-only: {e}"));
        Self {
            path: path.to_string(),
        }
    }
}

impl Drop for ReadOnlyDirectory {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(
            &self.path,
            std::fs::Permissions::from_mode(WRITABLE_DIRECTORY_MODE),
        );
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

static PREFLIGHT: Once = Once::new();

pub struct Config {
    base_url: String,
    secret: String,
    project_root: String,
    engine_model: String,
}

fn required_environment_variable(name: &str, expected_value: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} must be set to {expected_value}"))
}

impl Config {
    pub fn from_env() -> Self {
        let config = Config {
            base_url: required_environment_variable(
                BASE_URL_VARIABLE,
                "the main n8n instance's URL (e.g. http://localhost:5678)",
            ),
            secret: required_environment_variable(
                WEBHOOK_SECRET_VARIABLE,
                "the headless API's shared secret",
            ),
            project_root: required_environment_variable(
                PROJECT_ROOT_VARIABLE,
                &format!("the Eggocado-D2V checkout containing {OUTPUT_FOLDER}/ and {TEMP_FOLDER}/"),
            ),
            engine_model: required_environment_variable(
                ENGINE_MODEL_VARIABLE,
                "a transcription engine:model available on this instance (e.g. codex:gpt-5.6-sol, claude:claude-sonnet-5, or ollama:<model>)",
            ),
        };
        PREFLIGHT.call_once(|| assert_instances_are_ready_for_tests(&config));
        config
    }

    pub fn job_output_dir(&self, job_id: &str) -> String {
        format!("{}/{OUTPUT_FOLDER}/{job_id}", self.project_root)
    }

    pub fn job_temp_dir(&self, job_id: &str) -> String {
        format!("{}/{TEMP_FOLDER}/{job_id}", self.project_root)
    }

    pub fn final_video_path(&self, job_id: &str) -> String {
        format!("{}/{FINAL_VIDEO_FILE_NAME}", self.job_output_dir(job_id))
    }

    fn final_subtitles_path(&self, job_id: &str) -> String {
        format!("{}/{FINAL_SUBTITLES_FILE_NAME}", self.job_output_dir(job_id))
    }

    fn started_marker_path(&self, job_id: &str) -> String {
        format!("{}/{STARTED_MARKER_FILE_NAME}", self.job_output_dir(job_id))
    }

    pub fn ollama_model(&self) -> Option<&str> {
        self.engine_model.strip_prefix(OLLAMA_ENGINE_PREFIX)
    }
}

fn assert_status_request_passes_every_credential_on_its_path(config: &Config) {
    let status_response = send_status_request(config, PREFLIGHT_PROBE_JOB_ID).unwrap_or_else(|e| {
        panic!("failed to reach the status endpoint on {}: {e}", config.base_url)
    });
    let http_status = status_response.status();
    let response_body_text = status_response.text().unwrap_or_default();
    assert_eq!(
        http_status,
        StatusCode::OK,
        "a status request answered {http_status}: {response_body_text}. It passes {WEBHOOK_SECRET_VARIABLE} through the main workflow's status webhook, its {STATUS_CHECK_NODE} call and the bare-metal webhook that call reaches, so one of their credentials is missing or the secret is wrong"
    );
}

fn assert_start_request_passes_its_webhook_credential(config: &Config) {
    start_job_expecting_bad_request(config, &[]);
}

fn assert_video_inspection_programs_run() {
    for program in VIDEO_INSPECTION_PROGRAMS {
        let version_output = run_program(program, &[PROGRAM_VERSION_ARGUMENT]);
        assert!(
            version_output.status.success(),
            "{program} {PROGRAM_VERSION_ARGUMENT} failed, and the tests inspect every finished video with it: {}",
            String::from_utf8_lossy(&version_output.stderr)
        );
    }
}

fn assert_instances_are_ready_for_tests(config: &Config) {
    assert_video_inspection_programs_run();
    assert_status_request_passes_every_credential_on_its_path(config);
    assert_start_request_passes_its_webhook_credential(config);
}

pub fn job_id_for_document(document_filename: &str) -> String {
    let lowercase_filename = document_filename.to_lowercase();
    let document_name = lowercase_filename
        .strip_suffix(PDF_EXTENSION)
        .unwrap_or(&lowercase_filename);

    let mut slug = String::new();
    for character in document_name.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
        } else if !slug.ends_with(JOB_ID_WORD_SEPARATOR) {
            slug.push(JOB_ID_WORD_SEPARATOR);
        }
    }

    let slug = slug.trim_matches(JOB_ID_WORD_SEPARATOR);
    if slug.is_empty() {
        FALLBACK_JOB_ID.to_string()
    } else {
        slug.to_string()
    }
}

pub fn unique_document_filename(document_name_prefix: &str) -> String {
    let run_nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();
    format!("{document_name_prefix}-{run_nonce}{PDF_EXTENSION}")
}

fn page_selection_label(page: u32) -> String {
    format!("{PAGE_SELECTION_LABEL_PREFIX}{page}")
}

pub enum Field {
    Text {
        name: &'static str,
        value: String,
    },
    File {
        name: &'static str,
        filename: String,
        mime: &'static str,
        bytes: Vec<u8>,
    },
}

impl Field {
    pub fn text(name: &'static str, value: impl Into<String>) -> Self {
        Field::Text {
            name,
            value: value.into(),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Field::Text { name, .. } | Field::File { name, .. } => *name,
        }
    }
}

fn text_field_value<'a>(fields: &'a [Field], wanted_name: &str) -> Option<&'a str> {
    fields.iter().find_map(|field| match field {
        Field::Text { name, value } if *name == wanted_name => Some(value.as_str()),
        _ => None,
    })
}

pub fn without_fields(fields: Vec<Field>, removed_names: &[&str]) -> Vec<Field> {
    fields
        .into_iter()
        .filter(|field| !removed_names.contains(&field.name()))
        .collect()
}

pub fn replacing_field(fields: Vec<Field>, replacement: Field) -> Vec<Field> {
    let mut fields = without_fields(fields, &[replacement.name()]);
    fields.push(replacement);
    fields
}

pub fn uploaded_document_fields(config: &Config, document_filename: &str) -> Vec<Field> {
    vec![
        Field::text(field::MODE, mode::NEW),
        Field::text(field::DOCUMENT, document_filename),
        Field::text(field::ENGINE_MODEL, &config.engine_model),
        Field::text(field::VOICE, DEFAULT_VOICE),
        Field::File {
            name: field::UPLOADED_DOCUMENT,
            filename: document_filename.to_string(),
            mime: PDF_MIME_TYPE,
            bytes: generate_test_pdf(),
        },
    ]
}

pub fn new_conversion_fields(config: &Config, document_filename: &str) -> Vec<Field> {
    let mut fields = uploaded_document_fields(config, document_filename);
    fields.push(Field::text(field::OVERWRITE, OVERWRITE_CONFIRMED));
    fields
}

pub fn existing_document_fields(
    config: &Config,
    job_mode: &str,
    document_filename: &str,
    voice: &str,
) -> Vec<Field> {
    vec![
        Field::text(field::MODE, job_mode),
        Field::text(field::DOCUMENT, document_filename),
        Field::text(field::ENGINE_MODEL, &config.engine_model),
        Field::text(field::VOICE, voice),
    ]
}

pub fn knit_fields(
    config: &Config,
    document_filename: &str,
    left_page: u32,
    right_page: u32,
) -> Vec<Field> {
    let mut fields = existing_document_fields(config, mode::KNIT, document_filename, DEFAULT_VOICE);
    fields.push(Field::text(field::LEFT_PAGE, left_page.to_string()));
    fields.push(Field::text(field::RIGHT_PAGE, right_page.to_string()));
    fields
}

pub fn change_voice_fields(
    config: &Config,
    document_filename: &str,
    page: u32,
    target: &str,
) -> Vec<Field> {
    let mut fields =
        existing_document_fields(config, mode::CHANGE_VOICE, document_filename, ALTERNATE_VOICE);
    fields.push(Field::text(field::PAGE_SELECTION, page_selection_label(page)));
    fields.push(Field::text(field::TARGET, target));
    fields
}

pub fn sample_fields(sample_phrase: &str) -> Vec<Field> {
    vec![
        Field::text(field::MODE, mode::SAMPLE),
        Field::text(field::SAMPLE_PHRASE, sample_phrase),
    ]
}

#[derive(Debug, Deserialize)]
pub struct StartJobResponse {
    #[serde(rename = "jobId")]
    pub job_id: String,
    mode: Option<String>,
    #[serde(rename = "executionId")]
    execution_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    NotFound,
    Processing,
    Done,
    Failed(String),
}

#[derive(Debug, Deserialize)]
struct StatusResponse {
    status: String,
    message: Option<String>,
}

fn parse_job_status(status_response: StatusResponse) -> JobStatus {
    match status_response.status.as_str() {
        STATUS_NOT_FOUND => JobStatus::NotFound,
        STATUS_PROCESSING => JobStatus::Processing,
        STATUS_DONE => JobStatus::Done,
        STATUS_FAILED => JobStatus::Failed(status_response.message.unwrap_or_default()),
        other => panic!("status endpoint returned unknown status {other:?}"),
    }
}

fn send_start_request(config: &Config, fields: &[Field]) -> (StatusCode, String) {
    let multipart_form =
        fields
            .iter()
            .fold(reqwest::blocking::multipart::Form::new(), |form, field| match field {
                Field::Text { name, value } => form.text(*name, value.clone()),
                Field::File {
                    name,
                    filename,
                    mime,
                    bytes,
                } => {
                    let file_part = reqwest::blocking::multipart::Part::bytes(bytes.clone())
                        .file_name(filename.clone())
                        .mime_str(mime)
                        .expect("invalid mime type");
                    form.part(*name, file_part)
                }
            });

    let http_response = reqwest::blocking::Client::new()
        .post(format!("{}{START_JOB_PATH}", config.base_url))
        .header(WEBHOOK_SECRET_HEADER, &config.secret)
        .multipart(multipart_form)
        .send()
        .expect("failed to send start-job request");

    let http_status = http_response.status();
    let response_body_text = http_response
        .text()
        .expect("failed to read start-job response body");
    (http_status, response_body_text)
}

pub fn start_job_expecting_bad_request(config: &Config, fields: &[Field]) -> String {
    let (http_status, response_body_text) = send_start_request(config, fields);
    assert_eq!(
        http_status,
        StatusCode::BAD_REQUEST,
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

pub fn start_job_expecting_server_error(config: &Config, fields: &[Field]) -> String {
    let (http_status, response_body_text) = send_start_request(config, fields);
    assert_eq!(
        http_status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "expected HTTP 500 Internal Server Error, got {http_status}: {response_body_text}"
    );
    response_body_text
}

pub fn start_job(config: &Config, fields: &[Field]) -> StartJobResponse {
    let (http_status, response_body_text) = send_start_request(config, fields);
    assert!(
        http_status.is_success(),
        "start_job got HTTP {http_status}: {response_body_text}"
    );

    let started_job: StartJobResponse = serde_json::from_str(&response_body_text)
        .unwrap_or_else(|parse_error| {
            panic!("failed to parse start-job response: {parse_error}\nbody: {response_body_text}")
        });
    assert_start_response_matches_request(fields, &started_job);
    assert_started_marker_names_the_responding_execution(config, &started_job);
    started_job
}

pub fn start_new_conversion(config: &Config, document_filename: &str) -> StartJobResponse {
    start_job(config, &new_conversion_fields(config, document_filename))
}

fn assert_start_response_matches_request(fields: &[Field], started_job: &StartJobResponse) {
    let requested_mode = text_field_value(fields, field::MODE);
    assert_eq!(
        started_job.mode.as_deref(),
        requested_mode,
        "the start response names a different mode than the request"
    );

    let expected_job_id = if requested_mode == Some(mode::SAMPLE) {
        format!("{SAMPLE_JOB_ID_PREFIX}{}", started_job.execution_id)
    } else {
        job_id_for_document(text_field_value(fields, field::DOCUMENT).unwrap_or_default())
    };
    assert_eq!(
        started_job.job_id, expected_job_id,
        "the start response's jobId does not follow the documented naming rule"
    );
}

fn assert_started_marker_names_the_responding_execution(
    config: &Config,
    started_job: &StartJobResponse,
) {
    let job_id = &started_job.job_id;
    let marker_path = config.started_marker_path(job_id);
    let marker_contents = std::fs::read_to_string(&marker_path).unwrap_or_else(|e| {
        panic!("the start request for job {job_id} succeeded, but {marker_path} is unreadable: {e}")
    });
    assert_eq!(
        marker_contents.lines().last(),
        Some(format!("{STARTED_MARKER_EXECUTION_PREFIX}{}", started_job.execution_id).as_str()),
        "{marker_path} names a different execution than the one that answered the start request"
    );
}

pub fn assert_job_has_no_started_marker(config: &Config, job_id: &str) {
    let marker_path = config.started_marker_path(job_id);
    assert!(!Path::new(&marker_path).exists(), "{marker_path} exists");
}

fn send_status_request(
    config: &Config,
    job_id: &str,
) -> Result<reqwest::blocking::Response, reqwest::Error> {
    reqwest::blocking::Client::new()
        .get(format!("{}{JOB_STATUS_PATH}", config.base_url))
        .query(&[(JOB_ID_QUERY_PARAMETER, job_id)])
        .header(WEBHOOK_SECRET_HEADER, &config.secret)
        .send()
}

pub fn check_status_once(config: &Config, job_id: &str) -> Result<JobStatus, reqwest::Error> {
    let http_response = send_status_request(config, job_id)?;

    let response_body_text = http_response.text()?;
    let status_response: StatusResponse = serde_json::from_str(&response_body_text)
        .unwrap_or_else(|parse_error| {
            panic!("failed to parse status response: {parse_error}\nbody: {response_body_text}")
        });

    Ok(parse_job_status(status_response))
}

fn status_once_the_endpoint_answers(config: &Config, job_id: &str) -> JobStatus {
    wait_for(
        &format!("the status endpoint on {} to answer for job {job_id}", config.base_url),
        STATUS_UNREACHABLE_TIMEOUT,
        || check_status_once(config, job_id).ok(),
    )
}

pub fn finished_status_of_started_job(config: &Config, job_id: &str) -> Option<JobStatus> {
    match status_once_the_endpoint_answers(config, job_id) {
        JobStatus::NotFound => panic!(
            "job {job_id} wrote its started marker, but the status endpoint reports not_found"
        ),
        JobStatus::Processing => None,
        finished_status => Some(finished_status),
    }
}

pub fn poll_status(config: &Config, job_id: &str, timeout: Duration) -> JobStatus {
    poll_status_observing(config, job_id, timeout, || {})
}

pub fn poll_status_observing(
    config: &Config,
    job_id: &str,
    timeout: Duration,
    mut observe_each_poll: impl FnMut(),
) -> JobStatus {
    let poll_started_at = Instant::now();

    sleep(STATUS_POLL_INITIAL_DELAY);

    loop {
        observe_each_poll();

        if let Some(finished_status) = finished_status_of_started_job(config, job_id) {
            return finished_status;
        }

        if poll_started_at.elapsed() > timeout {
            panic!("job {job_id} did not reach done/failed within {timeout:?}");
        }

        sleep(STATUS_POLL_INTERVAL);
    }
}

pub fn assert_job_finishes(config: &Config, job_id: &str) {
    assert_eq!(
        poll_status(config, job_id, CONVERSION_TIMEOUT),
        JobStatus::Done,
        "job {job_id} did not finish"
    );
}

pub fn assert_request_fails_with_message_containing(
    config: &Config,
    fields: &[Field],
    timeout: Duration,
    expected_message_fragment: &str,
) {
    let job_id = start_job(config, fields).job_id;
    match poll_status(config, &job_id, timeout) {
        JobStatus::Failed(message) => assert!(
            message.contains(expected_message_fragment),
            "job {job_id} failed with {message:?}, which does not mention {expected_message_fragment:?}"
        ),
        other => panic!("expected job {job_id} to fail, got {other:?}"),
    }
}

pub fn ensure_document_converted(config: &Config, document_filename: &str) {
    let job_id = job_id_for_document(document_filename);
    if matches!(check_status_once(config, &job_id), Ok(JobStatus::Done)) {
        return;
    }

    start_new_conversion(config, document_filename);
    assert_job_finishes(config, &job_id);
}

pub fn assert_request_produces_a_playable_document_video(config: &Config, fields: &[Field]) {
    let started_job = start_job(config, fields);
    assert_job_finishes(config, &started_job.job_id);
    assert_final_video_is_playable(config, &started_job.job_id, DOCUMENT_VIDEO_DURATION_SECONDS);
}

fn wait_for<T>(awaited_value: &str, timeout: Duration, mut attempt: impl FnMut() -> Option<T>) -> T {
    let wait_started_at = Instant::now();

    loop {
        if let Some(value) = attempt() {
            return value;
        }
        if wait_started_at.elapsed() > timeout {
            panic!("waited {timeout:?} for {awaited_value}");
        }
        sleep(CONDITION_POLL_INTERVAL);
    }
}

pub fn wait_until(awaited_condition: &str, timeout: Duration, mut condition_holds: impl FnMut() -> bool) {
    wait_for(awaited_condition, timeout, || condition_holds().then_some(()))
}

pub fn something_listens_on(address: &str) -> bool {
    let socket_address: std::net::SocketAddr = address
        .parse()
        .unwrap_or_else(|e| panic!("invalid socket address {address}: {e}"));
    std::net::TcpStream::connect_timeout(&socket_address, SOCKET_CONNECT_TIMEOUT).is_ok()
}

pub fn wait_until_nothing_listens_on(address: &str, timeout: Duration) {
    wait_until(
        &format!("{address} to stop accepting connections"),
        timeout,
        || !something_listens_on(address),
    );
}

fn wait_for_file_size_to_stabilize(file_path: &str, timeout: Duration) {
    let wait_started_at = Instant::now();
    let mut last_seen_file_size = 0;
    let mut stable_reading_count = 0;

    loop {
        let current_file_size = std::fs::metadata(file_path)
            .unwrap_or_else(|e| panic!("failed to stat {file_path}: {e}"))
            .len();

        if current_file_size == last_seen_file_size && current_file_size > 0 {
            stable_reading_count += 1;
            if stable_reading_count >= STABLE_FILE_SIZE_READINGS_REQUIRED {
                return;
            }
        } else {
            stable_reading_count = 0;
        }
        last_seen_file_size = current_file_size;

        if wait_started_at.elapsed() > timeout {
            panic!("{file_path} did not stop growing within {timeout:?}");
        }

        sleep(FILE_SIZE_POLL_INTERVAL);
    }
}

fn generate_test_image_png_bytes() -> Vec<u8> {
    let band_count = FIXTURE_IMAGE_BAND_COLORS.len() as u32;
    let three_band_image = ::image::RgbImage::from_fn(
        FIXTURE_IMAGE_WIDTH_PIXELS,
        FIXTURE_IMAGE_HEIGHT_PIXELS,
        |x, _y| {
            let band_index = (x * band_count / FIXTURE_IMAGE_WIDTH_PIXELS) as usize;
            ::image::Rgb(FIXTURE_IMAGE_BAND_COLORS[band_index])
        },
    );

    let mut png_bytes = Vec::new();
    three_band_image
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), ::image::ImageFormat::Png)
        .expect("failed to encode synthesized test image as PNG");
    png_bytes
}

pub fn generate_test_pdf() -> Vec<u8> {
    let mut embedded_images = std::collections::BTreeMap::new();
    embedded_images.insert(
        FIXTURE_IMAGE_FILE_NAME.to_string(),
        Base64OrRaw::Raw(generate_test_image_png_bytes()),
    );
    let embedded_fonts = std::collections::BTreeMap::new();

    let options = GeneratePdfOptions {
        page_width: Some(A4_PAGE_WIDTH_MILLIMETERS),
        page_height: Some(A4_PAGE_HEIGHT_MILLIMETERS),
        margin_top: Some(FIXTURE_PAGE_MARGIN_MILLIMETERS),
        margin_right: Some(FIXTURE_PAGE_MARGIN_MILLIMETERS),
        margin_bottom: Some(FIXTURE_PAGE_MARGIN_MILLIMETERS),
        margin_left: Some(FIXTURE_PAGE_MARGIN_MILLIMETERS),
        ..Default::default()
    };

    let mut warnings = Vec::new();
    let pdf_document =
        PdfDocument::from_html(FIXTURE_HTML, &embedded_images, &embedded_fonts, &options, &mut warnings)
            .unwrap_or_else(|e| panic!("failed to generate fixture PDF from HTML: {e}"));

    pdf_document.save(&PdfSaveOptions::default(), &mut Vec::new())
}

fn run_program(program: &str, arguments: &[&str]) -> std::process::Output {
    Command::new(program)
        .args(arguments)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {program}: {e}"))
}

fn ffprobe(video_file_path: &str) -> serde_json::Value {
    let ffprobe_output = run_program(
        FFPROBE_PROGRAM,
        &[FFPROBE_JSON_REPORT_ARGUMENTS.as_slice(), &[video_file_path]].concat(),
    );

    assert!(
        ffprobe_output.status.success(),
        "{FFPROBE_PROGRAM} failed on {video_file_path}: {}",
        String::from_utf8_lossy(&ffprobe_output.stderr)
    );

    serde_json::from_slice(&ffprobe_output.stdout)
        .unwrap_or_else(|e| panic!("{FFPROBE_PROGRAM} produced invalid JSON: {e}"))
}

fn assert_video_has_audio_and_video_streams(video_file_path: &str) {
    let ffprobe_json = ffprobe(video_file_path);
    let media_streams = ffprobe_json[FFPROBE_STREAMS_KEY]
        .as_array()
        .unwrap_or_else(|| panic!("{FFPROBE_PROGRAM} output had no {FFPROBE_STREAMS_KEY} array"));
    for stream_type in [VIDEO_STREAM_TYPE, AUDIO_STREAM_TYPE] {
        assert!(
            media_streams
                .iter()
                .any(|stream| stream[FFPROBE_CODEC_TYPE_KEY] == stream_type),
            "{video_file_path} has no {stream_type} stream"
        );
    }
}

fn assert_video_duration_within_range(
    video_file_path: &str,
    expected_duration_seconds: RangeInclusive<f64>,
) {
    let ffprobe_json = ffprobe(video_file_path);
    let video_duration_seconds: f64 = ffprobe_json[FFPROBE_FORMAT_KEY][FFPROBE_DURATION_KEY]
        .as_str()
        .unwrap_or_else(|| panic!("{FFPROBE_PROGRAM} {FFPROBE_FORMAT_KEY}.{FFPROBE_DURATION_KEY} missing"))
        .parse()
        .unwrap_or_else(|e| panic!("{FFPROBE_PROGRAM} {FFPROBE_FORMAT_KEY}.{FFPROBE_DURATION_KEY} is not a number: {e}"));
    assert!(
        expected_duration_seconds.contains(&video_duration_seconds),
        "{video_file_path} duration {video_duration_seconds}s outside expected range {expected_duration_seconds:?}"
    );
}

fn assert_video_decodes_without_errors(video_file_path: &str) {
    let ffmpeg_decode_output = run_program(
        FFMPEG_PROGRAM,
        &[
            FFMPEG_DECODE_CHECK_ARGUMENTS_BEFORE_INPUT.as_slice(),
            &[video_file_path],
            FFMPEG_DECODE_CHECK_ARGUMENTS_AFTER_INPUT.as_slice(),
        ]
        .concat(),
    );
    assert!(
        ffmpeg_decode_output.status.success() && ffmpeg_decode_output.stderr.is_empty(),
        "{video_file_path} failed to decode cleanly: {}",
        String::from_utf8_lossy(&ffmpeg_decode_output.stderr)
    );
}

pub fn assert_final_video_is_playable(
    config: &Config,
    job_id: &str,
    expected_duration_seconds: RangeInclusive<f64>,
) {
    let video_path = config.final_video_path(job_id);
    wait_for_file_size_to_stabilize(&video_path, FILE_SIZE_STABILIZE_TIMEOUT);
    assert_video_has_audio_and_video_streams(&video_path);
    assert_video_duration_within_range(&video_path, expected_duration_seconds);
    assert_video_decodes_without_errors(&video_path);
}

pub fn assert_final_subtitles_are_well_formed(config: &Config, job_id: &str) {
    let srt_path = config.final_subtitles_path(job_id);
    let srt_file_contents = std::fs::read_to_string(&srt_path)
        .unwrap_or_else(|e| panic!("failed to read {srt_path}: {e}"));
    assert!(!srt_file_contents.trim().is_empty(), "{srt_path} is empty");
    assert_srt_has_well_formed_cues(&srt_path, &srt_file_contents);
}

fn assert_srt_has_well_formed_cues(srt_path: &str, srt_file_contents: &str) {
    let cue_timing_lines: Vec<&str> = srt_file_contents
        .lines()
        .filter(|line| line.contains(SRT_CUE_TIMING_SEPARATOR))
        .collect();
    assert!(!cue_timing_lines.is_empty(), "{srt_path} has no timestamp cues");

    for cue_timing_line in cue_timing_lines {
        let cue_timestamps: Vec<&str> = cue_timing_line.split(SRT_CUE_TIMING_SEPARATOR).collect();
        assert_eq!(
            cue_timestamps.len(),
            SRT_TIMESTAMPS_PER_CUE,
            "malformed srt timestamp line: {cue_timing_line}"
        );
        for timestamp_text in cue_timestamps {
            let trimmed_timestamp_text = timestamp_text.trim();
            assert!(
                trimmed_timestamp_text.len() == SRT_TIMESTAMP_LENGTH
                    && trimmed_timestamp_text.chars().nth(SRT_HOURS_MINUTES_SEPARATOR_INDEX)
                        == Some(SRT_TIME_SEPARATOR)
                    && trimmed_timestamp_text.chars().nth(SRT_SECONDS_MILLISECONDS_SEPARATOR_INDEX)
                        == Some(SRT_MILLISECONDS_SEPARATOR),
                "srt timestamp not in HH:MM:SS,mmm format: {trimmed_timestamp_text}"
            );
        }
    }
}
