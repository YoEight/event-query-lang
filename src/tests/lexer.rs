use crate::lexer::parse_tokens;

#[test]
fn test_lexer_all_kind() {
    insta::assert_yaml_snapshot!(parse_tokens("foo != 123(]{.:").unwrap());
}

#[test]
fn test_lexer_negative_number() {
    insta::assert_yaml_snapshot!(parse_tokens("-123.456").unwrap());
}
