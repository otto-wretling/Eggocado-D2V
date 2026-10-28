use std::time::Duration;

use d2v_headless_api_tests::{
    move_named_file_aside_in_every_subdirectory, start_job, Config, Field, JobStatus,
};

#[test]
fn starting_a_document_dependent_mode_with_no_documents_on_disk_fails_with_clear_message() {
    let config = Config::from_env();

    let meta_file_backups =
        move_named_file_aside_in_every_subdirectory(&config.temp_dir(), "meta.json");
    assert!(
        !meta_file_backups.is_empty(),
        "expected at least one existing document's meta.json to hide for this test"
    );

    let started_job = start_job(&config, &[Field::Text("mode", "knit")]);

    let job_status =
        d2v_headless_api_tests::poll_status(&config, &started_job.job_id, Duration::from_secs(30));
    match job_status {
        JobStatus::Failed(message) => {
            assert!(message.contains("documents"));
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}
