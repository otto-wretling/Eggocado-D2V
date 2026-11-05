use std::thread::sleep;
use std::time::Duration;

use d2v_headless_api_tests::{
    assert_job_finishes, assert_job_has_no_started_marker,
    assert_request_fails_with_message_containing, change_voice_fields, change_voice_target,
    check_status_once, discard_all_backups, ensure_document_converted, field,
    is_first_page_or_document_source, job_id_for_document, knit_fields, mode,
    move_entries_aside_except, new_conversion_fields, replacing_field,
    start_job_expecting_bad_request, start_job_expecting_server_error, start_new_conversion,
    unique_document_filename, uploaded_document_fields, wait_for_path_to_exist, without_fields,
    Config, Field, JobStatus, ReadOnlyDirectory, CONVERSION_TIMEOUT, FAILURE_POLL_TIMEOUT,
    FIRST_PAGE, SECOND_PAGE, SHARED_DOCUMENT_FILENAME,
};

const VALIDATION_PROBE_DOCUMENT_FILENAME: &str = "d2v-rust-suite-validation-probe.pdf";
const UNWRITABLE_OUTPUT_DOCUMENT_FILENAME: &str = "d2v-rust-suite-unwritable-output-folder.pdf";
const OVERWRITE_PRECONDITION_DOCUMENT_FILENAME: &str = "d2v-rust-suite-overwrite-precondition.pdf";
const NO_PAGES_YET_DOCUMENT_NAME_PREFIX: &str = "d2v-rust-suite-no-pages-yet-precondition";
const TOO_FEW_PAGES_DOCUMENT_NAME_PREFIX: &str = "d2v-rust-suite-too-few-pages-precondition";

const INVALID_FIELD_VALUE: &str = "bogus";
const TEXT_ABSENT_FROM_TRANSCRIPT: &str = "zzzznotpresentzzzz";
const NONEXISTENT_PAGE: u32 = 99;

const OVERWRITE_PRECONDITION_HEAD_START: Duration = Duration::from_secs(5);
const NO_PAGES_YET_PRECONDITION_HEAD_START: Duration = Duration::from_secs(1);
const NO_PAGES_YET_FAILURE_TIMEOUT: Duration = Duration::from_secs(180);

const TEXT_NOT_FOUND_MESSAGE_FRAGMENT: &str = "text";
const PAGE_NOT_FOUND_MESSAGE_FRAGMENT: &str = "page";
const NO_PROCESSED_PAGES_MESSAGE_FRAGMENT: &str = "processed pages";
const TOO_FEW_PAGES_MESSAGE_FRAGMENT: &str = "knit";
const MISSING_UPLOAD_MESSAGE_FRAGMENT: &str = "file";

fn assert_request_rejected_mentioning(config: &Config, fields: &[Field], expected_fragment: &str) {
    let error_message = start_job_expecting_bad_request(config, fields);
    assert!(
        error_message.contains(expected_fragment),
        "the rejection {error_message:?} does not mention {expected_fragment:?}"
    );
}

fn validation_probe_fields(config: &Config) -> Vec<Field> {
    new_conversion_fields(config, VALIDATION_PROBE_DOCUMENT_FILENAME)
}

#[test]
fn start_request_fails_when_the_job_output_folder_is_unwritable() {
    let config = Config::from_env();
    let job_id = job_id_for_document(UNWRITABLE_OUTPUT_DOCUMENT_FILENAME);
    let _unwritable_job_output_folder = ReadOnlyDirectory::create(&config.job_output_dir(&job_id));

    start_job_expecting_server_error(
        &config,
        &new_conversion_fields(&config, UNWRITABLE_OUTPUT_DOCUMENT_FILENAME),
    );

    assert_job_has_no_started_marker(&config, &job_id);
    assert_eq!(
        check_status_once(&config, &job_id).expect("failed to check status"),
        JobStatus::NotFound
    );
}

#[test]
fn missing_mode_fails_with_clear_message() {
    let config = Config::from_env();
    assert_request_rejected_mentioning(
        &config,
        &without_fields(validation_probe_fields(&config), &[field::MODE]),
        field::MODE,
    );
}

#[test]
fn invalid_mode_fails_with_clear_message() {
    let config = Config::from_env();
    assert_request_rejected_mentioning(
        &config,
        &replacing_field(
            validation_probe_fields(&config),
            Field::text(field::MODE, INVALID_FIELD_VALUE),
        ),
        field::MODE,
    );
}

#[test]
fn sample_mode_without_sample_phrase_fails_with_clear_message() {
    let config = Config::from_env();
    assert_request_rejected_mentioning(
        &config,
        &[Field::text(field::MODE, mode::SAMPLE)],
        field::SAMPLE_PHRASE,
    );
}

#[test]
fn knit_mode_without_document_fails_with_clear_message() {
    let config = Config::from_env();
    assert_request_rejected_mentioning(
        &config,
        &without_fields(
            knit_fields(&config, SHARED_DOCUMENT_FILENAME, FIRST_PAGE, SECOND_PAGE),
            &[field::DOCUMENT],
        ),
        field::DOCUMENT,
    );
}

#[test]
fn new_conversion_on_existing_document_without_overwrite_fails_with_clear_message() {
    let config = Config::from_env();

    start_new_conversion(&config, OVERWRITE_PRECONDITION_DOCUMENT_FILENAME);
    sleep(OVERWRITE_PRECONDITION_HEAD_START);

    assert_request_fails_with_message_containing(
        &config,
        &uploaded_document_fields(&config, OVERWRITE_PRECONDITION_DOCUMENT_FILENAME),
        FAILURE_POLL_TIMEOUT,
        field::OVERWRITE,
    );
}

#[test]
fn change_voice_with_text_not_in_transcript_fails_with_clear_message() {
    let config = Config::from_env();
    ensure_document_converted(&config, SHARED_DOCUMENT_FILENAME);

    let mut fields = change_voice_fields(
        &config,
        SHARED_DOCUMENT_FILENAME,
        FIRST_PAGE,
        change_voice_target::SELECTED_TEXT,
    );
    fields.push(Field::text(field::TEXT_TO_CHANGE, TEXT_ABSENT_FROM_TRANSCRIPT));

    assert_request_fails_with_message_containing(
        &config,
        &fields,
        FAILURE_POLL_TIMEOUT,
        TEXT_NOT_FOUND_MESSAGE_FRAGMENT,
    );
}

#[test]
fn change_voice_with_page_that_does_not_exist_fails_with_clear_message() {
    let config = Config::from_env();
    ensure_document_converted(&config, SHARED_DOCUMENT_FILENAME);

    assert_request_fails_with_message_containing(
        &config,
        &change_voice_fields(
            &config,
            SHARED_DOCUMENT_FILENAME,
            NONEXISTENT_PAGE,
            change_voice_target::WHOLE_PAGE,
        ),
        FAILURE_POLL_TIMEOUT,
        PAGE_NOT_FOUND_MESSAGE_FRAGMENT,
    );
}

#[test]
fn knit_mode_on_a_document_with_no_processed_pages_yet_fails_with_clear_message() {
    let config = Config::from_env();
    let document_filename = unique_document_filename(NO_PAGES_YET_DOCUMENT_NAME_PREFIX);

    start_new_conversion(&config, &document_filename);
    sleep(NO_PAGES_YET_PRECONDITION_HEAD_START);

    assert_request_fails_with_message_containing(
        &config,
        &knit_fields(&config, &document_filename, FIRST_PAGE, SECOND_PAGE),
        NO_PAGES_YET_FAILURE_TIMEOUT,
        NO_PROCESSED_PAGES_MESSAGE_FRAGMENT,
    );

    wait_for_path_to_exist(
        &config.final_video_path(&job_id_for_document(&document_filename)),
        CONVERSION_TIMEOUT,
    );
}

#[test]
fn knit_mode_with_too_few_pages_fails_with_clear_message() {
    let config = Config::from_env();
    let document_filename = unique_document_filename(TOO_FEW_PAGES_DOCUMENT_NAME_PREFIX);

    let setup_job = start_new_conversion(&config, &document_filename);
    assert_job_finishes(&config, &setup_job.job_id);

    let temp_backups = move_entries_aside_except(
        &config.job_temp_dir(&setup_job.job_id),
        is_first_page_or_document_source,
    );

    assert_request_fails_with_message_containing(
        &config,
        &knit_fields(&config, &document_filename, FIRST_PAGE, SECOND_PAGE),
        FAILURE_POLL_TIMEOUT,
        TOO_FEW_PAGES_MESSAGE_FRAGMENT,
    );

    discard_all_backups(temp_backups);
}

#[test]
fn knit_mode_without_left_and_right_page_fails_with_clear_message() {
    let config = Config::from_env();
    assert_request_rejected_mentioning(
        &config,
        &without_fields(
            knit_fields(&config, SHARED_DOCUMENT_FILENAME, FIRST_PAGE, SECOND_PAGE),
            &[field::LEFT_PAGE, field::RIGHT_PAGE],
        ),
        field::LEFT_PAGE,
    );
}

#[test]
fn change_voice_without_page_selection_or_target_fails_with_clear_message() {
    let config = Config::from_env();
    assert_request_rejected_mentioning(
        &config,
        &without_fields(
            change_voice_fields(
                &config,
                SHARED_DOCUMENT_FILENAME,
                FIRST_PAGE,
                change_voice_target::WHOLE_PAGE,
            ),
            &[field::PAGE_SELECTION, field::TARGET],
        ),
        field::PAGE_SELECTION,
    );
}

#[test]
fn change_voice_selected_text_without_text_to_change_fails_with_clear_message() {
    let config = Config::from_env();
    assert_request_rejected_mentioning(
        &config,
        &change_voice_fields(
            &config,
            SHARED_DOCUMENT_FILENAME,
            FIRST_PAGE,
            change_voice_target::SELECTED_TEXT,
        ),
        field::TEXT_TO_CHANGE,
    );
}

#[test]
fn missing_voice_fails_with_clear_message() {
    let config = Config::from_env();
    assert_request_rejected_mentioning(
        &config,
        &without_fields(validation_probe_fields(&config), &[field::VOICE]),
        field::VOICE,
    );
}

#[test]
fn missing_engine_model_fails_with_clear_message() {
    let config = Config::from_env();
    assert_request_rejected_mentioning(
        &config,
        &without_fields(validation_probe_fields(&config), &[field::ENGINE_MODEL]),
        field::ENGINE_MODEL,
    );
}

#[test]
fn invalid_acceleration_fails_with_clear_message() {
    let config = Config::from_env();
    let mut fields = validation_probe_fields(&config);
    fields.push(Field::text(field::ACCELERATION, INVALID_FIELD_VALUE));
    assert_request_rejected_mentioning(&config, &fields, field::ACCELERATION);
}

#[test]
fn new_conversion_without_file_fails_with_clear_message() {
    let config = Config::from_env();
    assert_request_rejected_mentioning(
        &config,
        &without_fields(validation_probe_fields(&config), &[field::UPLOADED_DOCUMENT]),
        MISSING_UPLOAD_MESSAGE_FRAGMENT,
    );
}
