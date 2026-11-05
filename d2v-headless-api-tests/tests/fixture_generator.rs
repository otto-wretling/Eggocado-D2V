const PDF_FILE_SIGNATURE: &[u8] = b"%PDF-";
const MINIMUM_FIXTURE_PDF_SIZE_BYTES: usize = 100;

#[test]
fn generated_pdf_is_non_empty_and_starts_with_pdf_header() {
    let pdf_bytes = d2v_headless_api_tests::generate_test_pdf();
    assert!(pdf_bytes.len() > MINIMUM_FIXTURE_PDF_SIZE_BYTES);
    assert!(pdf_bytes.starts_with(PDF_FILE_SIGNATURE));
}
