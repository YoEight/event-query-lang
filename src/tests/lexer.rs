use crate::Session;

#[test]
fn test_lexer_all_kind() {
    let session = Session::builder().build();
    insta::assert_yaml_snapshot!(session.tokenize("foo != 123(]{.:"));
}

#[test]
fn test_lexer_negative_number() {
    let session = Session::builder().build();
    insta::assert_yaml_snapshot!(session.tokenize("-123.456"));
}

#[test]
fn test_lexer_comment() {
    let session = Session::builder().build();
    insta::assert_yaml_snapshot!(session.tokenize("// useless comment\n "));
}
