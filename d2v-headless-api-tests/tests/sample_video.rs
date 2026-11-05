use std::thread::sleep;
use std::time::{Duration, Instant};

use d2v_headless_api_tests::{check_status_once, start_job, Config, Field, JobStatus, CONVERSION_TIMEOUT};

#[test]
fn sample_video_with_a_phrase_produces_at_least_one_voice_clip() {
    let config = Config::from_env();

    let started_job = start_job(
        &config,
        &[
            Field::Text("mode", "sample"),
            Field::Text(
                "samplePhrase",
                "Hello, this is {voice} speaking a sample phrase for the Rust test suite.",
            ),
        ],
    );

    assert_eq!(started_job.mode.as_deref(), Some("sample"));
    assert!(started_job.job_id.starts_with("sample-"));

    let job_id = &started_job.job_id;
    let job_temp_dir = config.job_temp_dir(job_id);
    let poll_started_at = Instant::now();
    let poll_timeout = CONVERSION_TIMEOUT;

    loop {
        let any_voice_clip_exists = std::fs::read_dir(&job_temp_dir)
            .map(|mut entries| {
                entries.any(|entry| {
                    entry
                        .map(|e| e.file_name().to_string_lossy().ends_with("_clip.mp4"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        if any_voice_clip_exists {
            return;
        }

        match check_status_once(&config, job_id) {
            Ok(JobStatus::Failed(message)) => {
                panic!("sample job {job_id} failed before producing a voice clip: {message}")
            }
            Ok(JobStatus::NotFound) => panic!(
                "sample job {job_id} wrote its started marker, but the status endpoint reports not_found"
            ),
            Ok(JobStatus::Done | JobStatus::Processing) | Err(_) => {}
        }

        if poll_started_at.elapsed() > poll_timeout {
            panic!("no voice clip appeared in {job_temp_dir} within {poll_timeout:?}");
        }

        sleep(Duration::from_secs(5));
    }
}
