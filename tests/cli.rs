#[test]
fn trycmd() {
    trycmd::TestCases::new().case("tests/cmd/*.toml");
}
