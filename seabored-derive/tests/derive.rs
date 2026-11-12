#[test]
fn derive_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/derive/pass/*.rs");
    t.compile_fail("tests/derive/fail/*.rs");
}
