use std::time::Duration;

use d2v_headless_api_tests::{start_job, Config, Field, JobStatus, TemporarilyMovedPath};

#[test]
fn continuing_a_document_with_its_pdf_missing_fails_with_clear_message() {
    let config = Config::from_env();
    let document_filename = "d2v-rust-suite-new-conversion.pdf";
    let pdf_path = format!("{}/{document_filename}", config.job_temp_dir("d2v-rust-suite-new-conversion"));

    let _pdf_backup_guard = TemporarilyMovedPath::move_aside_if_present(&pdf_path)
        .unwrap_or_else(|| panic!("expected {pdf_path} to exist before this test moved it aside"));

    let started_job = start_job(
        &config,
        &[
            Field::Text("mode", "continue"),
            Field::Text("document", document_filename),
            Field::Text("engineModel", "codex:gpt-5.6-sol"),
            Field::Text("voice", "af_heart"),
        ],
    );

    let job_status =
        d2v_headless_api_tests::poll_status(&config, &started_job.job_id, Duration::from_secs(30));
    match job_status {
        JobStatus::Failed(message) => {
            assert!(message.contains("PDF"));
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}
