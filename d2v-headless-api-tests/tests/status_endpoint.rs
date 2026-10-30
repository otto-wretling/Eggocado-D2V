use d2v_headless_api_tests::{check_status_once, Config, JobStatus};

#[test]
fn checking_status_of_a_never_used_job_id_reports_not_found() {
    let config = Config::from_env();

    let job_status = check_status_once(&config, "d2v-rust-suite-never-used-job-id-probe")
        .expect("failed to send status request");

    assert_eq!(job_status, JobStatus::NotFound);
}
