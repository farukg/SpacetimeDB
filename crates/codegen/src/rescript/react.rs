//! StdbReact.res + SpacetimeDBProvider.res — React hook bindings.
//!
//! Phantom types `query<'row>` and `reducerDef<'params>` enforce type identity
//! at the binding site: you cannot pass a receipt query where a booking query
//! is expected — the compiler rejects it.

use super::templates::{SpacetimedbProviderRes, StdbReactRes};
use crate::OutputFile;
use spacetimedb_schema::def::ModuleDef;

pub(super) fn generate_react_file(_module: &ModuleDef) -> Vec<OutputFile> {
    vec![
        OutputFile {
            filename: "StdbReact.res".to_string(),
            code: StdbReactRes.to_string(),
        },
        OutputFile {
            filename: "SpacetimeDBProvider.res".to_string(),
            code: SpacetimedbProviderRes.to_string(),
        },
    ]
}
