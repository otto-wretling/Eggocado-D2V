use std::time::Duration;

use d2v_headless_api_tests::{
    assert_video_decodes_without_errors, assert_video_duration_within_range,
    assert_video_has_audio_and_video_streams, start_job, wait_for_file_size_to_stabilize, Config,
    Field, JobStatus, CONVERSION_TIMEOUT,
};

#[test]
fn changing_voice_for_a_whole_page_produces_an_updated_playable_video() {
    let config = Config::from_env();

    let started_job = start_job(
        &config,
        &[
            Field::Text("mode", "changeVoice"),
            Field::Text("document", "d2v-rust-suite-new-conversion.pdf"),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("pageSelection", "Page 1"),
            Field::Text("target", "wholePage"),
            Field::Text("voice", "af_bella"),
        ],
    );

    assert_eq!(started_job.mode.as_deref(), Some("changeVoice"));
    assert_eq!(started_job.job_id, "d2v-rust-suite-new-conversion");

    let job_status = d2v_headless_api_tests::poll_status(
        &config,
        &started_job.job_id,
        CONVERSION_TIMEOUT,
    );
    assert_eq!(job_status, JobStatus::Done);

    let video_path = format!(
        "{}/final_video.mp4",
        config.job_output_dir(&started_job.job_id)
    );
    wait_for_file_size_to_stabilize(&video_path, Duration::from_secs(30));
    assert_video_has_audio_and_video_streams(&video_path);
    assert_video_duration_within_range(&video_path, 10.0, 600.0);
    assert_video_decodes_without_errors(&video_path);
}

#[test]
fn changing_voice_for_selected_text_produces_an_updated_playable_video() {
    let config = Config::from_env();

    let started_job = start_job(
        &config,
        &[
            Field::Text("mode", "changeVoice"),
            Field::Text("document", "d2v-rust-suite-new-conversion.pdf"),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("pageSelection", "Page 2"),
            Field::Text("target", "selectedText"),
            Field::Text("textToChange", "Product"),
            Field::Text("voice", "af_bella"),
        ],
    );

    assert_eq!(started_job.mode.as_deref(), Some("changeVoice"));
    assert_eq!(started_job.job_id, "d2v-rust-suite-new-conversion");

    let job_status = d2v_headless_api_tests::poll_status(
        &config,
        &started_job.job_id,
        CONVERSION_TIMEOUT,
    );
    assert_eq!(job_status, JobStatus::Done);

    let video_path = format!(
        "{}/final_video.mp4",
        config.job_output_dir(&started_job.job_id)
    );
    wait_for_file_size_to_stabilize(&video_path, Duration::from_secs(30));
    assert_video_has_audio_and_video_streams(&video_path);
    assert_video_duration_within_range(&video_path, 10.0, 600.0);
    assert_video_decodes_without_errors(&video_path);
}
