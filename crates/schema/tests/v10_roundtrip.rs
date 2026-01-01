use spacetimedb_lib::db::raw_def::v10::{
    CaseConversionPolicy, ExplicitNameEntry, ExplicitNames, MethodOrAny, RawModuleDefV10, RawModuleDefV10Builder,
};
use spacetimedb_lib::db::raw_def::v9::RawModuleDefV9Builder;
use spacetimedb_lib::{AlgebraicType, ProductType};
use spacetimedb_schema::def::ModuleDef;
use std::collections::BTreeSet;

fn type_names(module: &ModuleDef) -> BTreeSet<String> {
    module.types().map(|typ| typ.accessor_name.to_string()).collect()
}

fn assert_canonical_roundtrip(module: ModuleDef) {
    let expected_names = type_names(&module);
    let expected_typespace = module.typespace().clone();
    let raw: RawModuleDefV10 = module.into();
    assert!(matches!(raw.case_conversion_policy(), CaseConversionPolicy::None));
    let roundtrip: ModuleDef = raw.try_into().unwrap();
    assert_eq!(type_names(&roundtrip), expected_names);
    assert_eq!(roundtrip.typespace(), &expected_typespace);
}

#[test]
fn v9_canonical_names_survive_v10_export() {
    let mut builder = RawModuleDefV9Builder::new();
    let material = builder.add_algebraic_type(
        ["vault".into(), "inner_scope".into()],
        "secretMaterial",
        AlgebraicType::Product(ProductType::from([("CamelField", AlgebraicType::String)])),
        true,
    );
    builder.add_algebraic_type(
        [],
        "resultState",
        AlgebraicType::simple_enum(["ReadyValue", "missing_value"].into_iter()),
        true,
    );
    builder.add_reducer(
        "SubmitMaterial",
        ProductType::from([("CamelParam", AlgebraicType::Ref(material))]),
        None,
    );
    let module: ModuleDef = builder.finish().try_into().unwrap();
    assert_eq!(
        type_names(&module),
        BTreeSet::from(["resultState".into(), "vault::inner_scope::secretMaterial".into()])
    );
    let expected_reducers = module.reducers().cloned().collect::<Vec<_>>();
    let expected_typespace = module.typespace().clone();
    let raw: RawModuleDefV10 = module.into();
    assert!(matches!(raw.case_conversion_policy(), CaseConversionPolicy::None));
    let roundtrip: ModuleDef = raw.try_into().unwrap();
    assert_eq!(roundtrip.reducers().cloned().collect::<Vec<_>>(), expected_reducers);
    assert_eq!(roundtrip.typespace(), &expected_typespace);
    assert_eq!(
        type_names(&roundtrip),
        BTreeSet::from(["resultState".into(), "vault::inner_scope::secretMaterial".into()])
    );
    assert_canonical_roundtrip(roundtrip);
}

#[test]
fn fresh_v10_defaults_still_normalize_before_canonical_export() {
    let mut builder = RawModuleDefV10Builder::new();
    let material = builder.add_algebraic_type(
        ["vault".into()],
        "Secret",
        AlgebraicType::Product(ProductType::from([("camelField", AlgebraicType::String)])),
        true,
    );
    let raw = builder.finish();
    assert!(matches!(raw.case_conversion_policy(), CaseConversionPolicy::SnakeCase));
    let module: ModuleDef = raw.try_into().unwrap();
    assert_eq!(type_names(&module), BTreeSet::from(["Vault::Secret".into()]));
    let field = &module.typespace()[material].as_product().unwrap().elements[0];
    assert_eq!(field.name.as_ref().unwrap().as_ref(), "camel_field");
    assert_canonical_roundtrip(module);
}

#[test]
fn fresh_v10_http_handler_routes_preserve_canonical_names_on_export() {
    for (explicit_name, canonical_name) in [(None, "handle_request"), (Some("StableHandler"), "StableHandler")] {
        let mut builder = RawModuleDefV10Builder::new();
        builder.add_http_handler("HandleRequest");
        builder.add_http_route("HandleRequest", MethodOrAny::Any, "/roundtrip");
        if let Some(name) = explicit_name {
            let mut names = ExplicitNames::default();
            names.insert_function("HandleRequest", name);
            builder.add_explicit_names(names);
        }
        let original = builder.finish();
        assert!(matches!(
            original.case_conversion_policy(),
            CaseConversionPolicy::SnakeCase
        ));
        let module: ModuleDef = original.try_into().unwrap();
        let handler = module.http_handlers().next().unwrap();
        assert_eq!(handler.accessor_name.as_raw().as_ref(), "HandleRequest");
        assert_eq!(handler.name.as_raw().as_ref(), canonical_name);
        assert_eq!(module.http_routes()[0].handler_name.as_raw().as_ref(), canonical_name);
        let expected_handlers = module.http_handlers().cloned().collect::<Vec<_>>();
        let expected_routes = module.http_routes().to_vec();

        let raw: RawModuleDefV10 = module.into();
        assert!(matches!(raw.case_conversion_policy(), CaseConversionPolicy::None));
        assert_eq!(raw.http_handlers().unwrap()[0].source_name.as_ref(), "HandleRequest");
        assert_eq!(raw.http_routes().unwrap()[0].handler_function.as_ref(), "HandleRequest");
        assert!(raw
            .explicit_names()
            .unwrap()
            .clone()
            .into_entries()
            .iter()
            .any(|entry| {
                matches!(entry, ExplicitNameEntry::Function(mapping)
                if mapping.source_name.as_ref() == "HandleRequest"
                    && mapping.canonical_name.as_ref() == canonical_name)
            }));
        let roundtrip: ModuleDef = raw.try_into().unwrap();
        assert_eq!(
            roundtrip.http_handlers().cloned().collect::<Vec<_>>(),
            expected_handlers
        );
        assert_eq!(roundtrip.http_routes(), expected_routes.as_slice());
        assert_eq!(
            roundtrip.http_handlers().next().unwrap().name.as_raw().as_ref(),
            canonical_name
        );
        assert_eq!(
            roundtrip.http_routes()[0].handler_name.as_raw().as_ref(),
            canonical_name
        );
    }
}

#[test]
fn interleaved_http_route_order_survives_handler_grouped_export() {
    let mut builder = RawModuleDefV10Builder::new();
    for handler in ["FirstHandler", "UnusedHandler", "SecondHandler"] {
        builder.add_http_handler(handler);
    }
    for (handler, path) in [
        ("SecondHandler", "/second-first"),
        ("FirstHandler", "/first-first"),
        ("SecondHandler", "/second-later"),
        ("FirstHandler", "/first-later"),
    ] {
        builder.add_http_route(handler, MethodOrAny::Any, path);
    }
    let module: ModuleDef = builder.finish().try_into().unwrap();
    assert_eq!(
        module
            .http_routes()
            .iter()
            .map(|route| route.handler_name.as_raw().as_ref())
            .collect::<Vec<_>>(),
        vec!["second_handler", "first_handler", "second_handler", "first_handler"]
    );
    let expected_handlers = module.http_handlers().cloned().collect::<Vec<_>>();
    let expected_routes = module.http_routes().to_vec();
    assert_eq!(expected_handlers.len(), 3);

    let raw: RawModuleDefV10 = module.into();
    assert_eq!(
        raw.http_routes()
            .unwrap()
            .iter()
            .map(|route| (route.handler_function.as_ref(), route.path.as_ref()))
            .collect::<Vec<_>>(),
        vec![
            ("SecondHandler", "/second-first"),
            ("FirstHandler", "/first-first"),
            ("SecondHandler", "/second-later"),
            ("FirstHandler", "/first-later"),
        ]
    );
    let roundtrip: ModuleDef = raw.try_into().unwrap();
    assert_eq!(
        roundtrip.http_handlers().cloned().collect::<Vec<_>>(),
        expected_handlers
    );
    assert_eq!(roundtrip.http_routes(), expected_routes.as_slice());
}

#[test]
fn canonical_http_targets_are_exported_as_source_references() {
    let mut builder = RawModuleDefV10Builder::new();
    builder.add_http_handler("HandleRequest");
    builder.add_http_handler("handle_request");
    builder.add_http_route("HandleRequest", MethodOrAny::Any, "/first");
    builder.add_http_route("handle_request", MethodOrAny::Any, "/second");
    let mut names = ExplicitNames::default();
    names.insert_function("handle_request", "OtherEndpoint");
    builder.add_explicit_names(names);
    let module: ModuleDef = builder.finish().try_into().unwrap();
    assert_eq!(module.http_routes()[0].handler_name.as_raw().as_ref(), "handle_request");
    assert_eq!(module.http_routes()[1].handler_name.as_raw().as_ref(), "OtherEndpoint");
    let expected_handlers = module.http_handlers().cloned().collect::<Vec<_>>();
    let expected_routes = module.http_routes().to_vec();

    let raw: RawModuleDefV10 = module.into();
    assert_eq!(raw.http_routes().unwrap()[0].handler_function.as_ref(), "HandleRequest");
    assert_eq!(
        raw.http_routes().unwrap()[1].handler_function.as_ref(),
        "handle_request"
    );
    let roundtrip: ModuleDef = raw.try_into().unwrap();
    assert_eq!(
        roundtrip.http_handlers().cloned().collect::<Vec<_>>(),
        expected_handlers
    );
    assert_eq!(roundtrip.http_routes(), expected_routes.as_slice());
}
