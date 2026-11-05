use std::thread::sleep;
use std::time::{Duration, Instant};

use d2v_headless_api_tests::{
    check_status_once, generate_test_pdf, something_listens_on, start_job,
    wait_until_nothing_listens_on, Config, Field, JobStatus, CONVERSION_TIMEOUT,
};

const CPU_KOKORO_ADDRESS: &str = "127.0.0.1:8882";
const OLLAMA_RUNNING_MODELS_URL: &str = "http://127.0.0.1:11434/api/ps";
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(120);

fn ollama_model_is_loaded(model: &str) -> bool {
    let running_models: serde_json::Value = reqwest::blocking::get(OLLAMA_RUNNING_MODELS_URL)
        .and_then(|response| response.json())
        .unwrap_or_else(|e| panic!("failed to list Ollama's running models: {e}"));
    running_models["models"]
        .as_array()
        .is_some_and(|models| models.iter().any(|loaded| loaded["name"] == model))
}

fn wait_until_ollama_unloads(model: &str, timeout: Duration) {
    let wait_started_at = Instant::now();

    while ollama_model_is_loaded(model) {
        if wait_started_at.elapsed() > timeout {
            panic!("Ollama still had {model} loaded after {timeout:?}");
        }
        sleep(Duration::from_secs(2));
    }
}

fn poll_status_noting_whether_cpu_kokoro_answered(config: &Config, job_id: &str) -> (JobStatus, bool) {
    let poll_started_at = Instant::now();
    let mut cpu_kokoro_answered = false;

    sleep(Duration::from_secs(5));

    loop {
        cpu_kokoro_answered |= something_listens_on(CPU_KOKORO_ADDRESS);

        match check_status_once(config, job_id) {
            Ok(JobStatus::Done) => return (JobStatus::Done, cpu_kokoro_answered),
            Ok(JobStatus::Failed(message)) => {
                return (JobStatus::Failed(message), cpu_kokoro_answered)
            }
            Ok(JobStatus::NotFound) => panic!(
                "job {job_id} wrote its started marker, but the status endpoint reports not_found"
            ),
            Ok(JobStatus::Processing) | Err(_) => {}
        }

        if poll_started_at.elapsed() > CONVERSION_TIMEOUT {
            panic!("job {job_id} did not reach done/failed within {CONVERSION_TIMEOUT:?}");
        }

        sleep(Duration::from_secs(2));
    }
}

#[test]
fn cpu_acceleration_narrates_with_the_cpu_kokoro_instance_and_releases_cpu_services_afterward() {
    let config = Config::from_env();
    let document_filename = "d2v-rust-suite-cpu-acceleration.pdf";

    let started_job = start_job(
        &config,
        &[
            Field::Text("mode", "new"),
            Field::Text("document", document_filename),
            Field::Text("engineModel", &config.engine_model),
            Field::Text("voice", "af_heart"),
            Field::Text("overwrite", "true"),
            Field::Text("acceleration", "cpu"),
            Field::File {
                name: "data",
                filename: document_filename,
                mime: "application/pdf",
                bytes: generate_test_pdf(),
            },
        ],
    );
    assert_eq!(started_job.job_id, "d2v-rust-suite-cpu-acceleration");

    let (job_status, cpu_kokoro_answered) =
        poll_status_noting_whether_cpu_kokoro_answered(&config, &started_job.job_id);
    assert_eq!(job_status, JobStatus::Done);
    assert!(
        cpu_kokoro_answered,
        "the CPU Kokoro instance at {CPU_KOKORO_ADDRESS} never answered during the conversion"
    );

    wait_until_nothing_listens_on(CPU_KOKORO_ADDRESS, CLEANUP_TIMEOUT);

    if let Some(ollama_model) = config.engine_model.strip_prefix("ollama:") {
        wait_until_ollama_unloads(ollama_model, CLEANUP_TIMEOUT);
    }
}
