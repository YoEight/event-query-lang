use crate::Session;

#[test]
fn test_parse_from_events_nested_data() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!("./resources/from_events_nested_data.eql"));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parse_from_events_using_subquery() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!("./resources/from_events_using_subquery.eql"));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parse_from_events_where_subject_project_record_with_count() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!(
        "./resources/from_events_where_subject_project_record_with_count.eql"
    ));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parse_from_events_with_top_identity_projection() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!(
        "./resources/from_events_with_top_identity_projection.eql"
    ));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parse_from_events_with_type_to_project_record() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!(
        "./resources/from_events_with_type_to_project_record.eql"
    ));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parse_binary_op() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!("./resources/parser_binary_op.eql"));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parser_unhinged_unary_op() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!("./resources/parser_unhinged_unary_op.eql"));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parser_from_events_with_group_by_and_having() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!(
        "./resources/from_events_with_group_by_and_having.eql"
    ));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parser_from_events_with_distinct() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!("./resources/from_events_with_distinct.eql"));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parser_valid_contains() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!("./resources/valid_contains.eql"));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parser_valid_type_conversion() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!("./resources/valid_type_conversion.eql"));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parser_invalid_type_conversion_expr() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!("./resources/invalid_type_conversion_expr.eql"));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parser_with_comment() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!("./resources/with_comment.eql"));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}

#[test]
fn test_parser_order_by_no_ordering() {
    let mut session = Session::builder().build();
    let query = session.parse(include_str!("./resources/query_order_by_no_ordering.eql"));
    insta::assert_yaml_snapshot!(query.map(|q| q.view(&session.arena)));
}
