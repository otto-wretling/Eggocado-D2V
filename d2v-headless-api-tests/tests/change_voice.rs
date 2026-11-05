use d2v_headless_api_tests::{
    assert_request_produces_a_playable_document_video, change_voice_fields, change_voice_target,
    ensure_document_converted, field, Config, Field, FIRST_PAGE, SECOND_PAGE,
    SHARED_DOCUMENT_FILENAME, TEXT_ON_SECOND_PAGE,
};

#[test]
fn changing_voice_for_a_whole_page_produces_an_updated_playable_video() {
    let config = Config::from_env();
    ensure_document_converted(&config, SHARED_DOCUMENT_FILENAME);

    let fields = change_voice_fields(
        &config,
        SHARED_DOCUMENT_FILENAME,
        FIRST_PAGE,
        change_voice_target::WHOLE_PAGE,
    );
    assert_request_produces_a_playable_document_video(&config, &fields);
}

#[test]
fn changing_voice_for_selected_text_produces_an_updated_playable_video() {
    let config = Config::from_env();
    ensure_document_converted(&config, SHARED_DOCUMENT_FILENAME);

    let mut fields = change_voice_fields(
        &config,
        SHARED_DOCUMENT_FILENAME,
        SECOND_PAGE,
        change_voice_target::SELECTED_TEXT,
    );
    fields.push(Field::text(field::TEXT_TO_CHANGE, TEXT_ON_SECOND_PAGE));
    assert_request_produces_a_playable_document_video(&config, &fields);
}
