use crate::lexer::tokenize;

#[test]
fn test_lexer_all_kind() {
    insta::assert_yaml_snapshot!(tokenize("foo != 123(]{.:").unwrap());
}

#[test]
fn test_lexer_negative_number() {
    insta::assert_yaml_snapshot!(tokenize("-123.456").unwrap());
}
