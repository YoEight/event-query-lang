use crate::typing::analysis::AnalysisContext;
use crate::{Session, Type, parser::Parser};

#[test]
fn test_infer_wrong_where_clause_1() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/infer_wrong_where_clause_1.eql"));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_infer_wrong_where_clause_2() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/infer_wrong_where_clause_2.eql"));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_rename_duplicate_variable_names() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!(
        "./resources/rename_duplicate_variable_names.eql"
    ));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_rename_non_existing_variable() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/rename_non_existing_variable.eql"));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_rename_subquery() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/rename_subquery.eql"));

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(query.and_then(|q| {
            session
                .run_static_analysis(q)
                .map(|q| q.view(&session.arena))
        }))
    });
}

#[test]
fn test_analyze_valid_contains() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/valid_contains.eql"));

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(query.and_then(|q| {
            session
                .run_static_analysis(q)
                .map(|q| q.view(&session.arena))
        }));
    })
}

#[test]
fn test_analyze_invalid_type_contains() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/invalid_type_contains.eql"));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_valid_type_conversion() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/valid_type_conversion.eql"));
    insta::assert_yaml_snapshot!(query.map(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_valid_type_conversion_weird_case() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!(
        "./resources/valid_type_conversion-weird-case.eql"
    ));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_unknown_type_conversion() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse("FROM e IN events PROJECT INTO { value: e.data.value AS Foobar }");

    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }), @r###"
    Err:
      Analysis:
        UnknownType:
          - 1
          - 56
          - Foobar
    "###);
}

#[test]
fn test_analyze_prevent_using_aggregate_with_source_based_props() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!(
        "./resources/aggregate_with_sourced_bases_props.eql"
    ));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_valid_agg_usage() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/valid_agg_usage.eql"));

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(query.and_then(|q| {
            session
                .run_static_analysis(q)
                .map(|q| q.view(&session.arena))
        }))
    });
}

#[test]
fn test_analyze_reject_agg_in_predicate() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/reject_agg_in_predicate.eql"));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_agg_must_use_source_bound() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/agg_must_use_source_bound.eql"));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_optional_param_func() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/optional_param_func.eql"));

    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_typecheck_datetime_contravariance_1() {
    let mut session = Session::builder().use_stdlib().build();
    let tokens = session.tokenize("e.time").unwrap();
    let expr = Parser::new(&mut session.arena, tokens.as_slice())
        .parse_expr()
        .unwrap();

    let event_type = session.options.default_event_type;
    let mut analysis = session.analysis();

    analysis.test_declare("e", event_type);

    // `e.time` is a `Type::DateTime` but it will typecheck if a `Type::Date` is expected
    insta::assert_yaml_snapshot!(analysis.analyze_expr(
        &mut AnalysisContext::default(),
        expr,
        Type::Date
    ));
}

#[test]
fn test_typecheck_datetime_contravariance_2() {
    let mut session = Session::builder().use_stdlib().build();
    let tokens = session.tokenize("NOW()").unwrap();
    let expr = Parser::new(&mut session.arena, tokens.as_slice())
        .parse_expr()
        .unwrap();

    let mut analysis = session.analysis();

    // `NOW()` is a `Type::DateTime` but it will typecheck if a `Type::Time` is expected
    insta::assert_yaml_snapshot!(analysis.analyze_expr(
        &mut AnalysisContext::default(),
        expr,
        Type::Time
    ));
}

#[test]
fn test_typecheck_datetime_contravariance_3() {
    let mut session = Session::builder().use_stdlib().build();
    let tokens = session.tokenize("YEAR(NOW())").unwrap();
    let expr = Parser::new(&mut session.arena, tokens.as_slice())
        .parse_expr()
        .unwrap();

    let mut analysis = session.analysis();

    insta::assert_yaml_snapshot!(analysis.analyze_expr(
        &mut AnalysisContext::default(),
        expr,
        Type::Number
    ));
}

#[test]
fn test_typecheck_datetime_contravariance_4() {
    let mut session = Session::builder().use_stdlib().build();
    let tokens = session.tokenize("HOUR(NOW())").unwrap();
    let expr = Parser::new(&mut session.arena, tokens.as_slice())
        .parse_expr()
        .unwrap();

    let mut analysis = session.analysis();

    insta::assert_yaml_snapshot!(analysis.analyze_expr(
        &mut AnalysisContext::default(),
        expr,
        Type::Number
    ));
}

#[test]
fn test_analyze_allow_regular_property_project_into() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!(
        "./resources/allow_regular_property_project_into.eql"
    ));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_undeclared_variable_in_project_into_clause() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!(
        "./resources/undeclared_variable_in_project_into_clause.eql"
    ));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_lowercase_function() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/lowercase_function.eql"));

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(query.and_then(|q| {
            session
                .run_static_analysis(q)
                .map(|q| q.view(&session.arena))
        }))
    });
}

#[test]
fn test_analyze_project_agg_value() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/project_agg_value.eql"));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_reject_constant_expr_in_project_into_clause() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/reject_constant_expr.eql"));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_allow_constant_agg_func() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/allow_constant_agg_func.eql"));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_reject_group_by_with_order_by_no_agg() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!(
        "./resources/reject_group_by_with_order_by_no_agg.eql"
    ));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_accept_group_by_with_order_by_with_agg() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!(
        "./resources/accept_group_by_with_order_by_with_agg.eql"
    ));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_reject_group_by_no_agg() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/reject_group_by_no_agg.eql"));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_reject_group_by_no_agg_in_rec() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!(
        "./resources/reject_group_by_no_agg_in_rec.eql"
    ));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_analyze_accept_group_by_with_agg_rec() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/accept_group_by_with_agg_rec.eql"));

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(query.and_then(|q| {
            session
                .run_static_analysis(q)
                .map(|q| q.view(&session.arena))
        }))
    });
}

#[test]
fn test_reject_invalid_having_clause() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/reject_invalid_having_clause.eql"));
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_accept_valid_having_clause() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/valid_having_clause.eql"));

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(query.and_then(|q| {
            session
                .run_static_analysis(q)
                .map(|q| q.view(&session.arena))
        }))
    });
}

#[test]
fn test_ids_in_order_by_should_pass() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/ids_in_order_by_should_pass.eql"));

    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_ids_in_group_by_should_pass() {
    let mut session = Session::builder().use_stdlib().build();
    let query = session.parse(include_str!("./resources/ids_in_group_by_should_pass.eql"));

    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}

#[test]
fn test_project_event_decls() {
    let mut builder = Session::builder().use_stdlib();

    builder
        .declare_type()
        .define_record()
        .prop("id", Type::String)
        .prop("name", Type::String)
        .prop("version", Type::String)
        .prop("summary", Type::String)
        .prop("schema", Type::Unspecified)
        .for_data_source("command_decls");

    let mut session = builder.build();

    let query = session.parse(include_str!("./resources/project_event_decls.eql"));

    insta::assert_yaml_snapshot!(query.and_then(|q| {
        session
            .run_static_analysis(q)
            .map(|q| q.view(&session.arena))
    }));
}
