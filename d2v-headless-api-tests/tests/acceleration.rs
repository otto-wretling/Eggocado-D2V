use std::thread::sleep;
use std::time::Duration;

use d2v_headless_api_tests::{
    acceleration, assert_job_finishes, existing_document_fields, field,
    finished_status_of_started_job, mode, new_conversion_fields, poll_status,
    poll_status_observing, something_listens_on, start_job, wait_until,
    wait_until_nothing_listens_on, Config, Field, JobStatus, CONVERSION_TIMEOUT, DEFAULT_VOICE,
    FAILURE_POLL_TIMEOUT, SERVICE_RELEASE_TIMEOUT,
};

const CPU_KOKORO_ADDRESS: &str = "127.0.0.1:8882";
const OLLAMA_RUNNING_MODELS_URL: &str = "http://127.0.0.1:11434/api/ps";
const OLLAMA_RUNNING_MODELS_KEY: &str = "models";
const OLLAMA_MODEL_NAME_KEY: &str = "name";
const ERROR_HANDLER_SETTLE_TIME: Duration = Duration::from_secs(15);

const CPU_ACCELERATION_DOCUMENT_FILENAME: &str = "d2v-rust-suite-cpu-acceleration.pdf";
const SHARED_CPU_KOKORO_DOCUMENT_FILENAME: &str = "d2v-rust-suite-cpu-kokoro-shared.pdf";
const BYSTANDER_FAILURE_DOCUMENT_FILENAME: &str = "d2v-rust-suite-cpu-kokoro-bystander-failure.pdf";

fn ollama_model_is_loaded(model: &str) -> bool {
    let running_models: serde_json::Value = reqwest::blocking::get(OLLAMA_RUNNING_MODELS_URL)
        .and_then(|response| response.json())
        .unwrap_or_else(|e| panic!("failed to list Ollama's running models: {e}"));
    running_models[OLLAMA_RUNNING_MODELS_KEY]
        .as_array()
        .is_some_and(|models| models.iter().any(|loaded| loaded[OLLAMA_MODEL_NAME_KEY] == model))
}

fn start_cpu_conversion(config: &Config, document_filename: &str) -> String {
    let mut fields = new_conversion_fields(config, document_filename);
    fields.push(Field::text(field::ACCELERATION, acceleration::CPU));
    start_job(config, &fields).job_id
}

fn wait_until_cpu_kokoro_answers_while_job_runs(config: &Config, job_id: &str) {
    wait_until(
        &format!("the CPU Kokoro instance at {CPU_KOKORO_ADDRESS} to answer during job {job_id}"),
        CONVERSION_TIMEOUT,
        || {
            if let Some(finished_status) = finished_status_of_started_job(config, job_id) {
                panic!("job {job_id} reached {finished_status:?} before the CPU Kokoro instance at {CPU_KOKORO_ADDRESS} answered");
            }
            something_listens_on(CPU_KOKORO_ADDRESS)
        },
    );
}

fn assert_cpu_services_released(config: &Config) {
    wait_until_nothing_listens_on(CPU_KOKORO_ADDRESS, SERVICE_RELEASE_TIMEOUT);

    if let Some(ollama_model) = config.ollama_model() {
        wait_until(
            &format!("Ollama to unload {ollama_model}"),
            SERVICE_RELEASE_TIMEOUT,
            || !ollama_model_is_loaded(ollama_model),
        );
    }
}

#[test]
fn a_failing_job_leaves_the_cpu_kokoro_instance_running_for_a_cpu_conversion_in_progress() {
    let config = Config::from_env();

    let cpu_job_id = start_cpu_conversion(&config, SHARED_CPU_KOKORO_DOCUMENT_FILENAME);
    wait_until_cpu_kokoro_answers_while_job_runs(&config, &cpu_job_id);

    let failing_job = start_job(
        &config,
        &existing_document_fields(
            &config,
            mode::CONTINUE,
            BYSTANDER_FAILURE_DOCUMENT_FILENAME,
            DEFAULT_VOICE,
        ),
    );
    let failing_job_status = poll_status(&config, &failing_job.job_id, FAILURE_POLL_TIMEOUT);
    assert!(
        matches!(failing_job_status, JobStatus::Failed(_)),
        "expected the bystander job to fail, got {failing_job_status:?}"
    );
    sleep(ERROR_HANDLER_SETTLE_TIME);

    assert!(
        something_listens_on(CPU_KOKORO_ADDRESS),
        "the CPU Kokoro instance at {CPU_KOKORO_ADDRESS} stopped when job {} failed, while job {cpu_job_id} was still using it",
        failing_job.job_id
    );

    assert_job_finishes(&config, &cpu_job_id);
    assert_cpu_services_released(&config);
}

#[test]
fn cpu_acceleration_narrates_with_the_cpu_kokoro_instance_and_releases_cpu_services_afterward() {
    let config = Config::from_env();

    let job_id = start_cpu_conversion(&config, CPU_ACCELERATION_DOCUMENT_FILENAME);

    let mut cpu_kokoro_answered = false;
    let job_status = poll_status_observing(&config, &job_id, CONVERSION_TIMEOUT, || {
        cpu_kokoro_answered |= something_listens_on(CPU_KOKORO_ADDRESS);
    });
    assert_eq!(job_status, JobStatus::Done);
    assert!(
        cpu_kokoro_answered,
        "the CPU Kokoro instance at {CPU_KOKORO_ADDRESS} never answered during the conversion"
    );

    assert_cpu_services_released(&config);
}
