//! StdbReact.res + SpacetimeDBProvider.res — React hook bindings.
//!
//! Phantom types `query<'row>` and `reducerDef<'params>` enforce type identity
//! at the binding site: you cannot pass a receipt query where a booking query
//! is expected — the compiler rejects it.

use super::templates::{SpacetimedbProviderRes, StdbReactRes};
use crate::OutputFile;
use spacetimedb_schema::def::ModuleDef;

pub(super) fn generate_react_file(_module: &ModuleDef, root_module: &str) -> Vec<OutputFile> {
    let sdk_module = format!("{root_module}__Sdk");
    vec![
        OutputFile {
            filename: format!("{root_module}__React.res"),
            code: StdbReactRes.to_string(),
        },
        OutputFile {
            filename: format!("{root_module}__Provider.res"),
            code: SpacetimedbProviderRes {
                sdk_module: &sdk_module,
            }
            .to_string(),
        },
    ]
}
