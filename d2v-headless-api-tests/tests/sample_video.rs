use d2v_headless_api_tests::{
    assert_final_subtitles_are_well_formed, assert_final_video_is_playable, assert_job_finishes,
    sample_fields, start_job, Config, JobFilesRemovedOnDrop, SAMPLE_VIDEO_DURATION_SECONDS,
};

const SAMPLE_PHRASE: &str =
    "Hello, this is {voice} speaking a sample phrase for the Rust test suite.";

#[test]
fn sample_video_with_a_phrase_finishes_with_a_playable_video_and_matching_srt() {
    let config = Config::from_env();

    let started_job = start_job(&config, &sample_fields(SAMPLE_PHRASE));
    let _job_files = JobFilesRemovedOnDrop::for_job(&config, &started_job.job_id);
    assert_job_finishes(&config, &started_job.job_id);

    assert_final_video_is_playable(&config, &started_job.job_id, SAMPLE_VIDEO_DURATION_SECONDS);
    assert_final_subtitles_are_well_formed(&config, &started_job.job_id);
}
