use spacetimedb_codegen::{generate, CodegenOptions, Rust};
use spacetimedb_lib::db::raw_def::v9::RawModuleDefV9Builder;
use spacetimedb_lib::{AlgebraicType, ProductType};
use spacetimedb_schema::def::ModuleDef;
use std::collections::{BTreeMap, BTreeSet};

fn fixture() -> ModuleDef {
    let mut builder = RawModuleDefV9Builder::new();
    let record = builder.add_algebraic_type(
        ["example".into()],
        "Record",
        AlgebraicType::Product(ProductType::from([("value", AlgebraicType::String)])),
        true,
    );
    let envelope = builder.add_algebraic_type(
        [],
        "Envelope",
        AlgebraicType::Product(ProductType::from([
            ("record", AlgebraicType::Ref(record)),
            ("repeated", AlgebraicType::array(AlgebraicType::Ref(record))),
            ("optional", AlgebraicType::option(AlgebraicType::Ref(record))),
        ])),
        true,
    );
    let outcome = builder.add_algebraic_type(
        [],
        "Outcome",
        AlgebraicType::sum([
            ("Ready", AlgebraicType::Ref(envelope)),
            ("Missing", AlgebraicType::unit()),
        ]),
        true,
    );
    builder.add_algebraic_type(
        [],
        "State",
        AlgebraicType::simple_enum(["Active", "Inactive"].into_iter()),
        true,
    );
    builder.add_reducer(
        "submit",
        ProductType::from([("outcome", AlgebraicType::Ref(outcome))]),
        None,
    );
    builder.add_procedure("load", ProductType::unit(), AlgebraicType::Ref(outcome));
    builder.finish().try_into().unwrap()
}

fn files(module: &ModuleDef) -> BTreeMap<String, String> {
    generate(module, &Rust, &CodegenOptions::default())
        .unwrap()
        .into_iter()
        .map(|file| (file.filename, file.code))
        .collect()
}

#[test]
fn default_rust_projection_survives_v10_schema_roundtrip() {
    let original = fixture();
    let expected_names = BTreeSet::from([
        "Envelope".to_owned(),
        "Outcome".to_owned(),
        "State".to_owned(),
        "example::Record".to_owned(),
    ]);
    assert_eq!(
        original
            .types()
            .map(|typ| typ.accessor_name.to_string())
            .collect::<BTreeSet<_>>(),
        expected_names
    );
    let original_output = files(&original);
    let raw: spacetimedb_lib::db::raw_def::v10::RawModuleDefV10 = original.into();
    assert!(matches!(
        raw.case_conversion_policy(),
        spacetimedb_lib::db::raw_def::v10::CaseConversionPolicy::None
    ));
    let module: ModuleDef = raw.try_into().unwrap();
    assert_eq!(
        module
            .types()
            .map(|typ| typ.accessor_name.to_string())
            .collect::<BTreeSet<_>>(),
        expected_names
    );
    let projected = files(&module);
    assert_eq!(projected, original_output);
    for filename in [
        "example_record_type.rs",
        "envelope_type.rs",
        "outcome_type.rs",
        "state_type.rs",
    ] {
        assert!(
            projected[filename].contains("Deserialize, Clone, PartialEq, Debug)]"),
            "{filename}"
        );
    }
    assert!(projected["state_type.rs"].contains("#[derive(Copy, Eq, Hash)]"));
}

#[test]
fn unnamed_product_fields_are_rejected_by_existing_schema_contract() {
    let mut builder = RawModuleDefV9Builder::new();
    builder.add_algebraic_type(
        [],
        "TupleRecord",
        AlgebraicType::Product(ProductType::new(
            [spacetimedb_lib::ProductTypeElement::new(AlgebraicType::String, None)].into(),
        )),
        true,
    );
    let module: Result<ModuleDef, _> = builder.finish().try_into();
    spacetimedb_data_structures::expect_error_matching!(
        module,
        spacetimedb_schema::error::ValidationError::ClientCodegenError {
            error: spacetimedb_schema::type_for_generate::ClientCodegenError::NamelessTypeDefElement { .. },
            ..
        }
    );
}
