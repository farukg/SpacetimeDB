//! `{root}__Hooks.res` — observer-backed hooks + connection framework.
//!
//! Emitted when observer support is enabled. Provides:
//! - Connection context (`Provider`, `useConnection`, `useConnectionInfo`)
//! - Generic `useRows` hook (works with any table via `tableConfig`)
//! - Generic `useCallWith` / `useCallUnit` hooks (works with any reducer)
//! - `mkTable` constructor (used by `{root}__Bridge.res`)
//! - `Subscriptions` module (plain callbacks, no React)
//!
//! Does NOT import `@spacetimedb/rescript/react` — binds React primitives
//! directly, avoiding the module-init `createContext` crash on the server.

use super::templates::StdbHooksRes;
use crate::OutputFile;

pub(super) fn generate_hooks_file(root_module: &str) -> OutputFile {
    OutputFile {
        filename: format!("{root_module}__Hooks.res"),
        code: StdbHooksRes {
            root_module,
            fx_module: &format!("{root_module}__Fx"),
        }
        .to_string(),
    }
}
