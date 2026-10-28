use std::thread::sleep;
use std::time::{Duration, Instant};

use d2v_headless_api_tests::{start_job, Config, Field};

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
    assert_eq!(started_job.job_id, "sample");

    let job_temp_dir = config.job_temp_dir(&started_job.job_id);
    let poll_started_at = Instant::now();
    let poll_timeout = Duration::from_secs(10 * 60);

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

        if poll_started_at.elapsed() > poll_timeout {
            panic!("no voice clip appeared in {job_temp_dir} within {poll_timeout:?}");
        }

        sleep(Duration::from_secs(5));
    }
}
