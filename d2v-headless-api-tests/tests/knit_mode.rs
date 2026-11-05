use d2v_headless_api_tests::{
    assert_request_produces_a_playable_document_video, ensure_document_converted, knit_fields,
    Config, FIRST_PAGE, SECOND_PAGE, SHARED_DOCUMENT_FILENAME,
};

#[test]
fn knitting_pages_one_and_two_produces_an_updated_playable_video() {
    let config = Config::from_env();
    ensure_document_converted(&config, SHARED_DOCUMENT_FILENAME);

    let fields = knit_fields(&config, SHARED_DOCUMENT_FILENAME, FIRST_PAGE, SECOND_PAGE);
    assert_request_produces_a_playable_document_video(&config, &fields);
}
