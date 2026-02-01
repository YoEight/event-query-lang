use crate::lexer::tokenize;
use crate::parser::parse;

#[test]
fn test_parse_from_events_nested_data() {
    let tokens = tokenize(include_str!("./resources/from_events_nested_data.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parse_from_events_using_subquery() {
    let tokens = tokenize(include_str!("./resources/from_events_using_subquery.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parse_from_events_where_subject_project_record_with_count() {
    let tokens = tokenize(include_str!(
        "./resources/from_events_where_subject_project_record_with_count.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parse_from_events_with_top_identity_projection() {
    let tokens = tokenize(include_str!(
        "./resources/from_events_with_top_identity_projection.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parse_from_events_with_type_to_project_record() {
    let tokens = tokenize(include_str!(
        "./resources/from_events_with_type_to_project_record.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parse_binary_op() {
    let tokens = tokenize(include_str!("./resources/parser_binary_op.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parser_unhinged_unary_op() {
    let tokens = tokenize(include_str!("./resources/parser_unhinged_unary_op.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parser_from_events_with_group_by_and_having() {
    let tokens = tokenize(include_str!(
        "./resources/from_events_with_group_by_and_having.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parser_from_events_with_distinct() {
    let tokens = tokenize(include_str!("./resources/from_events_with_distinct.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parser_valid_contains() {
    let tokens = tokenize(include_str!("./resources/valid_contains.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parser_valid_type_conversion() {
    let tokens = tokenize(include_str!("./resources/valid_type_conversion.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parser_invalid_type_conversion_expr() {
    let tokens = tokenize(include_str!("./resources/invalid_type_conversion_expr.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parser_with_comment() {
    let tokens = tokenize(include_str!("./resources/with_comment.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}

#[test]
fn test_parser_order_by_no_ordering() {
    let tokens = tokenize(include_str!("./resources/query_order_by_no_ordering.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(tokens.as_slice()));
}
