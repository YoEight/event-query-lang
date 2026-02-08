use crate::arena::ExprArena;
use crate::lexer::tokenize;
use crate::parser::parse;

#[test]
fn test_parse_from_events_nested_data() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!("./resources/from_events_nested_data.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parse_from_events_using_subquery() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!("./resources/from_events_using_subquery.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parse_from_events_where_subject_project_record_with_count() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!(
        "./resources/from_events_where_subject_project_record_with_count.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parse_from_events_with_top_identity_projection() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!(
        "./resources/from_events_with_top_identity_projection.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parse_from_events_with_type_to_project_record() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!(
        "./resources/from_events_with_type_to_project_record.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parse_binary_op() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!("./resources/parser_binary_op.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parser_unhinged_unary_op() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!("./resources/parser_unhinged_unary_op.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parser_from_events_with_group_by_and_having() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!(
        "./resources/from_events_with_group_by_and_having.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parser_from_events_with_distinct() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!("./resources/from_events_with_distinct.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parser_valid_contains() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!("./resources/valid_contains.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parser_valid_type_conversion() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!("./resources/valid_type_conversion.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parser_invalid_type_conversion_expr() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!("./resources/invalid_type_conversion_expr.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parser_with_comment() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!("./resources/with_comment.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}

#[test]
fn test_parser_order_by_no_ordering() {
    let mut arena = ExprArena::default();
    let tokens = tokenize(include_str!("./resources/query_order_by_no_ordering.eql")).unwrap();
    insta::assert_yaml_snapshot!(parse(&mut arena, tokens.as_slice()).unwrap().view(&arena));
}
