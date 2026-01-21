use crate::{
    Type,
    lexer::tokenize,
    parse_query,
    parser::Parser,
    prelude::{Analysis, AnalysisContext, AnalysisOptions},
};

#[test]
fn test_infer_wrong_where_clause_1() {
    let query = parse_query(include_str!("./resources/infer_wrong_where_clause_1.eql")).unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_infer_wrong_where_clause_2() {
    let query = parse_query(include_str!("./resources/infer_wrong_where_clause_2.eql")).unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_rename_duplicate_variable_names() {
    let query = parse_query(include_str!(
        "./resources/rename_duplicate_variable_names.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_rename_non_existing_variable() {
    let query = parse_query(include_str!("./resources/rename_non_existing_variable.eql")).unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_rename_subquery() {
    let query = parse_query(include_str!("./resources/rename_subquery.eql")).unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_analyze_valid_contains() {
    let query = parse_query(include_str!("./resources/valid_contains.eql")).unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_analyze_invalid_type_contains() {
    let query = parse_query(include_str!("./resources/invalid_type_contains.eql")).unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_analyze_valid_type_conversion() {
    let query = parse_query(include_str!("./resources/valid_type_conversion.eql")).unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_analyze_invalid_type_conversion_custom_type() {
    let query = parse_query(include_str!("./resources/type_conversion_custom_type.eql")).unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_analyze_valid_type_conversion_custom_type() {
    let query = parse_query(include_str!("./resources/type_conversion_custom_type.eql")).unwrap();
    insta::assert_yaml_snapshot!(
        query.run_static_analysis(&AnalysisOptions::default().add_custom_type("Foobar"))
    );
}

#[test]
fn test_analyze_valid_type_conversion_weird_case() {
    let query = parse_query(include_str!(
        "./resources/valid_type_conversion-weird-case.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_analyze_prevent_using_aggregate_with_source_based_props() {
    let query = parse_query(include_str!(
        "./resources/aggregate_with_sourced_bases_props.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_analyze_valid_agg_usage() {
    let query = parse_query(include_str!("./resources/valid_agg_usage.eql")).unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_analyze_reject_agg_in_predicate() {
    let query = parse_query(include_str!("./resources/reject_agg_in_predicate.eql")).unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_analyze_agg_must_use_source_bound() {
    let query = parse_query(include_str!("./resources/agg_must_use_source_bound.eql")).unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_analyze_optional_param_func() {
    let query = parse_query(include_str!("./resources/optional_param_func.eql")).unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_typecheck_datetime_contravariance_1() {
    let tokens = tokenize("e.time").unwrap();
    let expr = Parser::new(tokens.as_slice()).parse_expr().unwrap();
    let options = &AnalysisOptions::default();
    let mut analysis = Analysis::new(&options);

    analysis
        .scope_mut()
        .entries
        .insert("e".to_string(), options.event_type_info.clone());

    // `e.time` is a `Type::DateTime` but it will typecheck if a `Type::Date` is expected
    insta::assert_yaml_snapshot!(analysis.analyze_expr(
        &AnalysisContext::default(),
        &expr,
        Type::Date
    ));
}

#[test]
fn test_typecheck_datetime_contravariance_2() {
    let tokens = tokenize("NOW()").unwrap();
    let expr = Parser::new(tokens.as_slice()).parse_expr().unwrap();
    let options = &AnalysisOptions::default();
    let mut analysis = Analysis::new(&options);

    // `NOW()` is a `Type::DateTime` but it will typecheck if a `Type::Time` is expected
    insta::assert_yaml_snapshot!(analysis.analyze_expr(
        &AnalysisContext::default(),
        &expr,
        Type::Time
    ));
}

#[test]
fn test_typecheck_datetime_contravariance_3() {
    let tokens = tokenize("YEAR(NOW())").unwrap();
    let expr = Parser::new(tokens.as_slice()).parse_expr().unwrap();
    let options = &AnalysisOptions::default();
    let mut analysis = Analysis::new(&options);

    insta::assert_yaml_snapshot!(analysis.analyze_expr(
        &AnalysisContext::default(),
        &expr,
        Type::Number
    ));
}

#[test]
fn test_typecheck_datetime_contravariance_4() {
    let tokens = tokenize("HOUR(NOW())").unwrap();
    let expr = Parser::new(tokens.as_slice()).parse_expr().unwrap();
    let options = &AnalysisOptions::default();
    let mut analysis = Analysis::new(&options);

    insta::assert_yaml_snapshot!(analysis.analyze_expr(
        &AnalysisContext::default(),
        &expr,
        Type::Number
    ));
}

#[test]
fn test_analyze_allow_regular_property_project_into() {
    let query = parse_query(include_str!(
        "./resources/allow_regular_property_project_into.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}

#[test]
fn test_analyze_undeclared_variable_in_project_into_clause() {
    let query = parse_query(include_str!(
        "./resources/undeclared_variable_in_project_into_clause.eql"
    ))
    .unwrap();
    insta::assert_yaml_snapshot!(query.run_static_analysis(&Default::default()));
}
