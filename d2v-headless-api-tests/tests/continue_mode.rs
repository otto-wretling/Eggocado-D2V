use std::time::Duration;

use d2v_headless_api_tests::{
    assert_srt_has_well_formed_cues, assert_srt_is_not_empty, assert_video_decodes_without_errors,
    assert_video_duration_within_range, assert_video_has_audio_and_video_streams,
    discard_all_backups, generate_test_pdf, move_entries_aside_except, move_every_entry_aside,
    start_job, wait_for_file_size_to_stabilize, Config, Field, JobStatus,
};

#[test]
fn continuing_an_interrupted_conversion_resumes_from_page_two() {
    let config = Config::from_env();
    let document_filename = "d2v-rust-suite-continue-mode.pdf";

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
                bytes: generate_test_pdf(),
            },
        ],
    );

    let job_status =
        d2v_headless_api_tests::poll_status(&config, &started_job.job_id, Duration::from_secs(2 * 60 * 60));
    assert_eq!(job_status, JobStatus::Done);

    let job_temp_dir = config.job_temp_dir(&started_job.job_id);
    let job_output_dir = config.job_output_dir(&started_job.job_id);

    let temp_backups = move_entries_aside_except(&job_temp_dir, |entry_name| {
        entry_name.starts_with("1_") || entry_name == "meta.json" || entry_name.ends_with(".pdf")
    });
    let output_backups = move_every_entry_aside(&job_output_dir);

    let continued_job = start_job(
        &config,
        &[
            Field::Text("mode", "continue"),
            Field::Text("document", document_filename),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("voice", "af_heart"),
        ],
    );
    assert_eq!(continued_job.mode.as_deref(), Some("continue"));
    assert_eq!(continued_job.job_id, started_job.job_id);

    let continued_status = d2v_headless_api_tests::poll_status(
        &config,
        &continued_job.job_id,
        Duration::from_secs(2 * 60 * 60),
    );
    assert_eq!(continued_status, JobStatus::Done);

    discard_all_backups(temp_backups);
    discard_all_backups(output_backups);

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

    assert!(std::path::Path::new(&format!("{job_temp_dir}/2_clip.mp4")).exists());
    assert!(std::path::Path::new(&format!("{job_temp_dir}/3_clip.mp4")).exists());
}
