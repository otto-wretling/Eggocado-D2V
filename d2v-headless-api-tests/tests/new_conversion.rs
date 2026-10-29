use std::time::Duration;

use d2v_headless_api_tests::{
    assert_srt_has_well_formed_cues, assert_srt_is_not_empty, assert_video_decodes_without_errors,
    assert_video_duration_within_range, assert_video_has_audio_and_video_streams,
    generate_test_pdf, start_job, wait_for_file_size_to_stabilize, Config, Field, JobStatus,
    CONVERSION_TIMEOUT,
};

#[test]
fn new_conversion_produces_a_playable_video_and_matching_srt() {
    let config = Config::from_env();
    let document_filename = "d2v-rust-suite-new-conversion.pdf";
    let pdf_bytes = generate_test_pdf();

    let started_job = start_job(
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
                bytes: pdf_bytes,
            },
        ],
    );

    assert_eq!(started_job.mode.as_deref(), Some("new"));
    assert_eq!(started_job.job_id, "d2v-rust-suite-new-conversion");

    let job_status = d2v_headless_api_tests::poll_status(
        &config,
        &started_job.job_id,
        CONVERSION_TIMEOUT,
    );
    assert_eq!(job_status, JobStatus::Done);

    let job_output_dir = config.job_output_dir(&started_job.job_id);
    let video_path = format!("{job_output_dir}/final_video.mp4");
    let srt_path = format!("{job_output_dir}/final_video.srt");

    wait_for_file_size_to_stabilize(&video_path, Duration::from_secs(30));
    assert_video_has_audio_and_video_streams(&video_path);
    assert_video_duration_within_range(&video_path, 10.0, 600.0);
    assert_video_decodes_without_errors(&video_path);

    let srt_file_contents = std::fs::read_to_string(&srt_path)
        .unwrap_or_else(|e| panic!("failed to read {srt_path}: {e}"));
    assert_srt_is_not_empty(&srt_file_contents);
    assert_srt_has_well_formed_cues(&srt_file_contents);
}
