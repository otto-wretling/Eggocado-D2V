use std::time::Duration;

use d2v_headless_api_tests::{start_job, Config, Field, JobStatus};

const FAILURE_POLL_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn sample_mode_without_sample_phrase_fails_with_clear_message() {
    let config = Config::from_env();

    let started_job = start_job(&config, &[Field::Text("mode", "sample")]);

    let job_status = d2v_headless_api_tests::poll_status(
        &config,
        &started_job.job_id,
        FAILURE_POLL_TIMEOUT,
    );
    match job_status {
        JobStatus::Failed(message) => {
            assert!(message.contains("samplePhrase"));
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn knit_mode_without_document_fails_with_clear_message() {
    let config = Config::from_env();

    start_job(
        &config,
        &[
            Field::Text("mode", "new"),
            Field::Text("document", "d2v-rust-suite-document-required-precondition.pdf"),
            Field::Text("engineModel", "codex:gpt-5.6-sol"),
            Field::Text("voice", "af_heart"),
            Field::Text("overwrite", "true"),
            Field::File {
                name: "data",
                filename: "d2v-rust-suite-document-required-precondition.pdf",
                mime: "application/pdf",
                bytes: d2v_headless_api_tests::generate_test_pdf(),
            },
        ],
    );
    std::thread::sleep(Duration::from_secs(5));

    let started_job = start_job(&config, &[Field::Text("mode", "knit")]);

    let job_status = d2v_headless_api_tests::poll_status(
        &config,
        &started_job.job_id,
        FAILURE_POLL_TIMEOUT,
    );
    match job_status {
        JobStatus::Failed(message) => {
            assert!(message.contains("document"));
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn new_conversion_on_existing_document_without_overwrite_fails_with_clear_message() {
    let config = Config::from_env();
    let document_filename = "d2v-rust-suite-overwrite-precondition.pdf";

    start_job(
        &config,
        &[
            Field::Text("mode", "new"),
            Field::Text("document", document_filename),
            Field::Text("engineModel", "codex:gpt-5.6-sol"),
            Field::Text("voice", "af_heart"),
            Field::Text("overwrite", "true"),
            Field::File {
                name: "data",
                filename: document_filename,
                mime: "application/pdf",
                bytes: d2v_headless_api_tests::generate_test_pdf(),
            },
        ],
    );
    std::thread::sleep(Duration::from_secs(5));

    let started_job = start_job(
        &config,
        &[
            Field::Text("mode", "new"),
            Field::Text("document", document_filename),
            Field::Text("engineModel", "codex:gpt-5.6-sol"),
            Field::Text("voice", "af_heart"),
            Field::File {
                name: "data",
                filename: document_filename,
                mime: "application/pdf",
                bytes: d2v_headless_api_tests::generate_test_pdf(),
            },
        ],
    );

    let job_status = d2v_headless_api_tests::poll_status(
        &config,
        &started_job.job_id,
        FAILURE_POLL_TIMEOUT,
    );
    match job_status {
        JobStatus::Failed(message) => {
            assert!(message.contains("overwrite"));
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}
