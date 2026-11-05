use std::path::Path;

use d2v_headless_api_tests::{
    assert_final_subtitles_are_well_formed, assert_final_video_is_playable, assert_job_finishes,
    discard_all_backups, existing_document_fields, is_first_page_or_document_source, mode,
    move_entries_aside_except, move_every_entry_aside, page_clip_file_name, start_job,
    start_new_conversion, Config, DEFAULT_VOICE, DOCUMENT_VIDEO_DURATION_SECONDS, SECOND_PAGE,
};

const DOCUMENT_FILENAME: &str = "d2v-rust-suite-continue-mode.pdf";
const THIRD_PAGE: u32 = 3;

#[test]
fn continuing_an_interrupted_conversion_resumes_from_page_two() {
    let config = Config::from_env();

    let job_id = start_new_conversion(&config, DOCUMENT_FILENAME).job_id;
    assert_job_finishes(&config, &job_id);

    let job_temp_dir = config.job_temp_dir(&job_id);
    let temp_backups = move_entries_aside_except(&job_temp_dir, is_first_page_or_document_source);
    let output_backups = move_every_entry_aside(&config.job_output_dir(&job_id));

    let continued_job = start_job(
        &config,
        &existing_document_fields(&config, mode::CONTINUE, DOCUMENT_FILENAME, DEFAULT_VOICE),
    );
    assert_job_finishes(&config, &continued_job.job_id);

    discard_all_backups(temp_backups);
    discard_all_backups(output_backups);

    assert_final_video_is_playable(&config, &job_id, DOCUMENT_VIDEO_DURATION_SECONDS);
    assert_final_subtitles_are_well_formed(&config, &job_id);

    for resumed_page in [SECOND_PAGE, THIRD_PAGE] {
        let clip_path = format!("{job_temp_dir}/{}", page_clip_file_name(resumed_page));
        assert!(
            Path::new(&clip_path).exists(),
            "continue mode did not recreate {clip_path}"
        );
    }
}
