use d2v_headless_api_tests::{check_status_once, Config, JobStatus};

const NEVER_USED_JOB_ID: &str = "d2v-rust-suite-never-used-job-id-probe";

#[test]
fn checking_status_of_a_never_used_job_id_reports_not_found() {
    let config = Config::from_env();

    let job_status =
        check_status_once(&config, NEVER_USED_JOB_ID).expect("failed to send status request");

    assert_eq!(job_status, JobStatus::NotFound);
}
