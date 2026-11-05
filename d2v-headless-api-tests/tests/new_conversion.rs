use d2v_headless_api_tests::{
    assert_final_subtitles_are_well_formed, assert_final_video_is_playable, assert_job_finishes,
    start_new_conversion, Config, DOCUMENT_VIDEO_DURATION_SECONDS, SHARED_DOCUMENT_FILENAME,
};

#[test]
fn new_conversion_produces_a_playable_video_and_matching_srt() {
    let config = Config::from_env();

    let started_job = start_new_conversion(&config, SHARED_DOCUMENT_FILENAME);
    assert_job_finishes(&config, &started_job.job_id);

    assert_final_video_is_playable(&config, &started_job.job_id, DOCUMENT_VIDEO_DURATION_SECONDS);
    assert_final_subtitles_are_well_formed(&config, &started_job.job_id);
}
