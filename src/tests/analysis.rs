use crate::{
    Type,
    arena::ExprArena,
    lexer::tokenize,
    parser::Parser,
    prelude::{Analysis, AnalysisContext, AnalysisOptions},
};

#[test]
fn test_infer_wrong_where_clause_1() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/infer_wrong_where_clause_1.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_infer_wrong_where_clause_2() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/infer_wrong_where_clause_2.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_rename_duplicate_variable_names() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/rename_duplicate_variable_names.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_rename_non_existing_variable() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/rename_non_existing_variable.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_rename_subquery() {
    let mut arena = ExprArena::default();
    let query = parse_query(&mut arena, include_str!("./resources/rename_subquery.eql"));

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(query.and_then(|q| {
            q.run_static_analysis(&arena, &Default::default())
                .map(|q| q.view(&arena))
        }))
    });
}

#[test]
fn test_analyze_valid_contains() {
    let mut arena = ExprArena::default();
    let query = parse_query(&mut arena, include_str!("./resources/valid_contains.eql"));

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(query.and_then(|q| {
            q.run_static_analysis(&arena, &Default::default())
                .map(|q| q.view(&arena))
        }));
    })
}

#[test]
fn test_analyze_invalid_type_contains() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/invalid_type_contains.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_valid_type_conversion() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/valid_type_conversion.eql"),
    );
    insta::assert_yaml_snapshot!(query.map(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_invalid_type_conversion_custom_type() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/type_conversion_custom_type.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_valid_type_conversion_custom_type() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/type_conversion_custom_type.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(
            &arena,
            &AnalysisOptions::default().add_custom_type("Foobar"),
        )
        .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_valid_type_conversion_weird_case() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/valid_type_conversion-weird-case.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_prevent_using_aggregate_with_source_based_props() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/aggregate_with_sourced_bases_props.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_valid_agg_usage() {
    let mut arena = ExprArena::default();
    let query = parse_query(&mut arena, include_str!("./resources/valid_agg_usage.eql"));

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(query.and_then(|q| {
            q.run_static_analysis(&arena, &Default::default())
                .map(|q| q.view(&arena))
        }))
    });
}

#[test]
fn test_analyze_reject_agg_in_predicate() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/reject_agg_in_predicate.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_agg_must_use_source_bound() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/agg_must_use_source_bound.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_optional_param_func() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/optional_param_func.eql"),
    );

    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_typecheck_datetime_contravariance_1() {
    let mut arena = ExprArena::default();
    let tokens = tokenize("e.time").unwrap();
    let expr = Parser::new(&mut arena, tokens.as_slice())
        .parse_expr()
        .unwrap();
    let options = &AnalysisOptions::default();
    let mut analysis = Analysis::new(&arena, &options);

    analysis
        .scope_mut()
        .entries
        .insert("e".to_string(), options.event_type_info.clone());

    // `e.time` is a `Type::DateTime` but it will typecheck if a `Type::Date` is expected
    insta::assert_yaml_snapshot!(analysis.analyze_expr(
        &mut AnalysisContext::default(),
        expr,
        Type::Date
    ));
}

#[test]
fn test_typecheck_datetime_contravariance_2() {
    let mut arena = ExprArena::default();
    let tokens = tokenize("NOW()").unwrap();
    let expr = Parser::new(&mut arena, tokens.as_slice())
        .parse_expr()
        .unwrap();
    let options = &AnalysisOptions::default();
    let mut analysis = Analysis::new(&arena, &options);

    // `NOW()` is a `Type::DateTime` but it will typecheck if a `Type::Time` is expected
    insta::assert_yaml_snapshot!(analysis.analyze_expr(
        &mut AnalysisContext::default(),
        expr,
        Type::Time
    ));
}

#[test]
fn test_typecheck_datetime_contravariance_3() {
    let mut arena = ExprArena::default();
    let tokens = tokenize("YEAR(NOW())").unwrap();
    let expr = Parser::new(&mut arena, tokens.as_slice())
        .parse_expr()
        .unwrap();
    let options = &AnalysisOptions::default();
    let mut analysis = Analysis::new(&arena, &options);

    insta::assert_yaml_snapshot!(analysis.analyze_expr(
        &mut AnalysisContext::default(),
        expr,
        Type::Number
    ));
}

#[test]
fn test_typecheck_datetime_contravariance_4() {
    let mut arena = ExprArena::default();
    let tokens = tokenize("HOUR(NOW())").unwrap();
    let expr = Parser::new(&mut arena, tokens.as_slice())
        .parse_expr()
        .unwrap();
    let options = &AnalysisOptions::default();
    let mut analysis = Analysis::new(&arena, &options);

    insta::assert_yaml_snapshot!(analysis.analyze_expr(
        &mut AnalysisContext::default(),
        expr,
        Type::Number
    ));
}

#[test]
fn test_analyze_allow_regular_property_project_into() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/allow_regular_property_project_into.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_undeclared_variable_in_project_into_clause() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/undeclared_variable_in_project_into_clause.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_lowercase_function() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/lowercase_function.eql"),
    );

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(query.and_then(|q| {
            q.run_static_analysis(&arena, &Default::default())
                .map(|q| q.view(&arena))
        }))
    });
}

#[test]
fn test_analyze_project_agg_value() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/project_agg_value.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_reject_constant_expr_in_project_into_clause() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/reject_constant_expr.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_allow_constant_agg_func() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/allow_constant_agg_func.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_reject_group_by_with_order_by_no_agg() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/reject_group_by_with_order_by_no_agg.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_accept_group_by_with_order_by_with_agg() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/accept_group_by_with_order_by_with_agg.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_reject_group_by_no_agg() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/reject_group_by_no_agg.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_reject_group_by_no_agg_in_rec() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/reject_group_by_no_agg_in_rec.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_analyze_accept_group_by_with_agg_rec() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/accept_group_by_with_agg_rec.eql"),
    );

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(query.and_then(|q| {
            q.run_static_analysis(&arena, &Default::default())
                .map(|q| q.view(&arena))
        }))
    });
}

#[test]
fn test_reject_invalid_having_clause() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/reject_invalid_having_clause.eql"),
    );
    insta::assert_yaml_snapshot!(query.and_then(|q| {
        q.run_static_analysis(&arena, &Default::default())
            .map(|q| q.view(&arena))
    }));
}

#[test]
fn test_accept_valid_having_clause() {
    let mut arena = ExprArena::default();
    let query = parse_query(
        &mut arena,
        include_str!("./resources/valid_having_clause.eql"),
    );

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(query.and_then(|q| {
            q.run_static_analysis(&arena, &Default::default())
                .map(|q| q.view(&arena))
        }))
    });
}
