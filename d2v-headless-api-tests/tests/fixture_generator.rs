#[test]
fn generated_pdf_is_non_empty_and_starts_with_pdf_header() {
    let pdf_bytes = d2v_headless_api_tests::generate_test_pdf();
    assert!(pdf_bytes.len() > 100);
    assert_eq!(&pdf_bytes[0..5], b"%PDF-");
}
