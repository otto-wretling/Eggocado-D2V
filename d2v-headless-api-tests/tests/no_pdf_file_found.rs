use d2v_headless_api_tests::{
    assert_request_fails_with_message_containing, existing_document_fields, job_id_for_document,
    mode, Config, TemporarilyMovedPath, DEFAULT_VOICE, FAILURE_POLL_TIMEOUT,
    SHARED_DOCUMENT_FILENAME,
};

const MISSING_PDF_MESSAGE_FRAGMENT: &str = "PDF";

#[test]
fn continuing_a_document_with_its_pdf_missing_fails_with_clear_message() {
    let config = Config::from_env();
    let pdf_path = format!(
        "{}/{SHARED_DOCUMENT_FILENAME}",
        config.job_temp_dir(&job_id_for_document(SHARED_DOCUMENT_FILENAME))
    );

    let _pdf_backup_guard = TemporarilyMovedPath::move_aside_if_present(&pdf_path)
        .unwrap_or_else(|| panic!("expected {pdf_path} to exist before this test moved it aside"));

    assert_request_fails_with_message_containing(
        &config,
        &existing_document_fields(&config, mode::CONTINUE, SHARED_DOCUMENT_FILENAME, DEFAULT_VOICE),
        FAILURE_POLL_TIMEOUT,
        MISSING_PDF_MESSAGE_FRAGMENT,
    );
}
