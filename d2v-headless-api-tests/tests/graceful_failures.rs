use std::time::Duration;

use d2v_headless_api_tests::{
    start_job, start_job_expecting_bad_request, Config, Field, JobStatus, CONVERSION_TIMEOUT,
};

const FAILURE_POLL_TIMEOUT: Duration = Duration::from_secs(60);

#[test]
fn missing_mode_fails_with_clear_message() {
    let config = Config::from_env();

    let error_message = start_job_expecting_bad_request(
        &config,
        &[Field::Text(
            "document",
            "d2v-rust-suite-missing-mode-precondition.pdf",
        )],
    );

    assert!(error_message.contains("mode"));
}

#[test]
fn invalid_mode_fails_with_clear_message() {
    let config = Config::from_env();

    let error_message = start_job_expecting_bad_request(
        &config,
        &[
            Field::Text("mode", "bogus"),
            Field::Text("document", "d2v-rust-suite-invalid-mode-precondition.pdf"),
        ],
    );

    assert!(error_message.contains("mode"));
}

#[test]
fn sample_mode_without_sample_phrase_fails_with_clear_message() {
    let config = Config::from_env();

    let error_message =
        start_job_expecting_bad_request(&config, &[Field::Text("mode", "sample")]);

    assert!(error_message.contains("samplePhrase"));
}

#[test]
fn knit_mode_without_document_fails_with_clear_message() {
    let config = Config::from_env();

    let error_message = start_job_expecting_bad_request(
        &config,
        &[
            Field::Text("mode", "knit"),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("voice", "af_heart"),
            Field::Text("leftPage", "1"),
            Field::Text("rightPage", "2"),
        ],
    );

    assert!(error_message.contains("document"));
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
            Field::Text("engineModel", &config.engine_model),
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
            Field::Text("engineModel", &config.engine_model),
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

#[test]
fn change_voice_with_text_not_in_transcript_fails_with_clear_message() {
    let config = Config::from_env();

    let started_job = start_job(
        &config,
        &[
            Field::Text("mode", "changeVoice"),
            Field::Text("document", "d2v-rust-suite-new-conversion.pdf"),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("pageSelection", "Page 1"),
            Field::Text("target", "selectedText"),
            Field::Text("textToChange", "zzzznotpresentzzzz"),
            Field::Text("voice", "af_bella"),
        ],
    );

    let job_status = d2v_headless_api_tests::poll_status(
        &config,
        &started_job.job_id,
        FAILURE_POLL_TIMEOUT,
    );
    match job_status {
        JobStatus::Failed(message) => {
            assert!(message.contains("text"));
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn change_voice_with_page_that_does_not_exist_fails_with_clear_message() {
    let config = Config::from_env();

    let started_job = start_job(
        &config,
        &[
            Field::Text("mode", "changeVoice"),
            Field::Text("document", "d2v-rust-suite-new-conversion.pdf"),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("pageSelection", "Page 99"),
            Field::Text("target", "wholePage"),
            Field::Text("voice", "af_bella"),
        ],
    );

    let job_status = d2v_headless_api_tests::poll_status(
        &config,
        &started_job.job_id,
        FAILURE_POLL_TIMEOUT,
    );
    match job_status {
        JobStatus::Failed(message) => {
            assert!(message.contains("page"));
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn knit_mode_on_a_document_with_no_processed_pages_yet_fails_with_clear_message() {
    let config = Config::from_env();
    let run_nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();
    let document_filename =
        format!("d2v-rust-suite-no-pages-yet-precondition-{run_nonce}.pdf");

    start_job(
        &config,
        &[
            Field::Text("mode", "new"),
            Field::Text("document", &document_filename),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("voice", "af_heart"),
            Field::Text("overwrite", "true"),
            Field::File {
                name: "data",
                filename: &document_filename,
                mime: "application/pdf",
                bytes: d2v_headless_api_tests::generate_test_pdf(),
            },
        ],
    );
    std::thread::sleep(Duration::from_secs(1));

    let started_job = start_job(
        &config,
        &[
            Field::Text("mode", "knit"),
            Field::Text("document", &document_filename),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("leftPage", "1"),
            Field::Text("rightPage", "2"),
            Field::Text("voice", "af_heart"),
        ],
    );

    let job_status = d2v_headless_api_tests::poll_status(
        &config,
        &started_job.job_id,
        Duration::from_secs(180),
    );
    match job_status {
        JobStatus::Failed(message) => {
            assert!(message.contains("processed pages"));
        }
        other => panic!("expected Failed, got {other:?}"),
    }

    d2v_headless_api_tests::wait_for_path_to_exist(
        &format!(
            "{}/final_video.mp4",
            config.job_output_dir(&started_job.job_id)
        ),
        CONVERSION_TIMEOUT,
    );
}

#[test]
fn knit_mode_with_too_few_pages_fails_with_clear_message() {
    let config = Config::from_env();

    let started_job = start_job(
        &config,
        &[
            Field::Text("mode", "knit"),
            Field::Text("document", "d2v-jobid-match-test.pdf"),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("leftPage", "1"),
            Field::Text("rightPage", "2"),
            Field::Text("voice", "af_heart"),
        ],
    );

    let job_status = d2v_headless_api_tests::poll_status(
        &config,
        &started_job.job_id,
        FAILURE_POLL_TIMEOUT,
    );
    match job_status {
        JobStatus::Failed(message) => {
            assert!(message.contains("knit"));
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn knit_mode_without_left_and_right_page_fails_with_clear_message() {
    let config = Config::from_env();

    let error_message = start_job_expecting_bad_request(
        &config,
        &[
            Field::Text("mode", "knit"),
            Field::Text("document", "d2v-rust-suite-new-conversion.pdf"),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("voice", "af_heart"),
        ],
    );

    assert!(error_message.contains("leftPage"));
}

#[test]
fn change_voice_without_page_selection_or_target_fails_with_clear_message() {
    let config = Config::from_env();

    let error_message = start_job_expecting_bad_request(
        &config,
        &[
            Field::Text("mode", "changeVoice"),
            Field::Text("document", "d2v-rust-suite-new-conversion.pdf"),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("voice", "af_heart"),
        ],
    );

    assert!(error_message.contains("pageSelection"));
}

#[test]
fn change_voice_selected_text_without_text_to_change_fails_with_clear_message() {
    let config = Config::from_env();

    let error_message = start_job_expecting_bad_request(
        &config,
        &[
            Field::Text("mode", "changeVoice"),
            Field::Text("document", "d2v-rust-suite-new-conversion.pdf"),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("voice", "af_heart"),
            Field::Text("pageSelection", "Page 1"),
            Field::Text("target", "selectedText"),
        ],
    );

    assert!(error_message.contains("textToChange"));
}

#[test]
fn missing_voice_fails_with_clear_message() {
    let config = Config::from_env();

    let error_message = start_job_expecting_bad_request(
        &config,
        &[
            Field::Text("mode", "new"),
            Field::Text("document", "d2v-rust-suite-missing-voice-precondition.pdf"),
            Field::Text("engineModel", &config.engine_model),
        ],
    );

    assert!(error_message.contains("voice"));
}

#[test]
fn missing_engine_model_fails_with_clear_message() {
    let config = Config::from_env();

    let error_message = start_job_expecting_bad_request(
        &config,
        &[
            Field::Text("mode", "new"),
            Field::Text(
                "document",
                "d2v-rust-suite-missing-engine-model-precondition.pdf",
            ),
            Field::Text("voice", "af_heart"),
        ],
    );

    assert!(error_message.contains("engineModel"));
}

#[test]
fn new_conversion_without_file_fails_with_clear_message() {
    let config = Config::from_env();

    let error_message = start_job_expecting_bad_request(
        &config,
        &[
            Field::Text("mode", "new"),
            Field::Text("document", "d2v-rust-suite-missing-file-precondition.pdf"),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("voice", "af_heart"),
        ],
    );

    assert!(error_message.contains("file"));
}
