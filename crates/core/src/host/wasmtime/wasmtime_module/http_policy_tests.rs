use super::*;

fn import_module(engine: &wasmtime::Engine, namespace: &str, name: &str) -> wasmtime::Module {
    let mut bytes = b"\0asm\x01\0\0\0".to_vec();
    bytes.extend_from_slice(&[1, 10, 1, 0x60, 5, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 1, 0x7f]);
    let mut imports = vec![1, u8::try_from(namespace.len()).unwrap()];
    imports.extend_from_slice(namespace.as_bytes());
    imports.push(u8::try_from(name.len()).unwrap());
    imports.extend_from_slice(name.as_bytes());
    imports.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&[2, u8::try_from(imports.len()).unwrap()]);
    bytes.extend(imports);
    wasmtime::Module::new(engine, bytes).unwrap()
}

#[test]
fn host_10_5_rejects_strict_module_at_production_preinstantiation_gate() {
    let engine = wasmtime::Engine::default();
    let module = import_module(&engine, "spacetime_10.6", "procedure_http_request_with_policy");
    let linker = Linker::<WasmInstanceEnv>::new(&engine);
    let error = instantiate_pre_checked(&linker, &module, abi::VersionTuple::new(10, 5))
        .err()
        .unwrap();
    assert!(matches!(
        error.downcast_ref::<abi::AbiVersionError>(),
        Some(abi::AbiVersionError::UnsupportedVersion { .. }),
    ));
    assert!(!error.to_string().contains("SYNTHETIC_SECRET_MARKER"));
}

#[test]
fn current_host_links_both_legacy_and_strict_imports_on_both_lanes() {
    let sync_engine = wasmtime::Engine::default();
    let mut async_config = wasmtime::Config::new();
    async_config.async_support(true);
    let async_engine = wasmtime::Engine::new(&async_config).unwrap();
    for (engine, mode) in [
        (&sync_engine, AsyncImportMode::SyncStub),
        (&async_engine, AsyncImportMode::Async),
    ] {
        let mut linker = Linker::<WasmInstanceEnv>::new(engine);
        link_imports(&mut linker, mode).unwrap();
        for (namespace, name) in [
            ("spacetime_10.3", "procedure_http_request"),
            ("spacetime_10.6", "procedure_http_request_with_policy"),
        ] {
            let module = import_module(engine, namespace, name);
            assert!(instantiate_pre_checked(&linker, &module, WasmtimeModule::IMPLEMENTED_ABI).is_ok());
        }
    }
}

#[test]
fn registered_policy_import_has_its_own_mandatory_minor_version() {
    macro_rules! sync_imports {
        ($($namespace:literal :: $function:ident,)*) => {
            const SYNC: &[(&str, &str)] = &[$(($namespace, stringify!($function))),*];
        };
    }
    macro_rules! async_imports {
        ($($namespace:literal :: $function:ident,)*) => {
            const ASYNC: &[(&str, &str)] = &[$(($namespace, stringify!($function))),*];
        };
    }
    crate::host::wasm_common::abi_funcs!(sync_imports, async_imports);
    assert!(ASYNC.contains(&("spacetime_10.6", "procedure_http_request_with_policy")));
    assert!(ASYNC.contains(&("spacetime_10.3", "procedure_http_request")));
    assert!(!SYNC
        .iter()
        .any(|(_, name)| *name == "procedure_http_request_with_policy"));
    let required = abi::determine_spacetime_abi(ASYNC, |import| import.0).unwrap();
    assert!(abi::verify_supported(abi::VersionTuple::new(10, 5), required).is_err());
    assert!(abi::verify_supported(WasmtimeModule::IMPLEMENTED_ABI, required).is_ok());
}

#[test]
fn actor_factory_validates_before_actor_or_database_initialization() {
    let source = include_str!("../mod.rs");
    let gate = source.find("wasmtime_module::instantiate_pre_checked(").unwrap();
    let actor = source.find("WasmModuleHostActor::new(mcc, module)").unwrap();
    assert!(gate < actor);
}
