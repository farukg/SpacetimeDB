#[cfg(feature = "commands")]
pub mod api;
#[cfg(feature = "commands")]
#[doc(hidden)]
pub mod build_support;
#[cfg(feature = "commands")]
mod common_args;
mod config;
#[cfg(feature = "commands")]
pub(crate) mod detect;
#[cfg(feature = "commands")]
mod edit_distance;
mod errors;
#[cfg(feature = "commands")]
mod schema_extract;
pub mod server_url;
pub mod spacetime_config;
#[cfg(feature = "commands")]
mod subcommands;
#[cfg(feature = "commands")]
mod tasks;
#[cfg(feature = "commands")]
pub mod util;
#[cfg(feature = "commands")]
pub mod version;

#[cfg(feature = "commands")]
use std::process::ExitCode;

#[cfg(feature = "commands")]
use clap::{ArgMatches, Command};

pub use config::Config;
#[cfg(feature = "commands")]
use spacetimedb_paths::{RootDir, SpacetimePaths};
#[cfg(feature = "commands")]
pub use subcommands::*;
#[cfg(feature = "commands")]
pub use tasks::build;

#[cfg(feature = "commands")]
pub fn get_subcommands() -> Vec<Command> {
    vec![
        publish::cli(),
        env::cli(),
        delete::cli(),
        logs::cli(),
        call::cli(),
        describe::cli(),
        dev::cli(),
        sql::cli(),
        mcp::cli(),
        dns::cli(),
        generate::cli(),
        list::cli(),
        login::cli(),
        logout::cli(),
        init::cli(),
        build::cli(),
        server::cli(),
        subscribe::cli(),
        start::cli(),
        lock::cli(),
        unlock::cli(),
        auth::cli(),
        subcommands::version::cli(),
    ]
}

#[cfg(feature = "commands")]
pub async fn exec_subcommand(
    config: Config,
    paths: &SpacetimePaths,
    root_dir: Option<&RootDir>,
    cmd: &str,
    args: &ArgMatches,
) -> anyhow::Result<ExitCode> {
    match cmd {
        "call" => call::exec(config, args).await,
        "describe" => describe::exec(config, args).await,
        "dev" => dev::exec(config, args).await,
        "publish" => publish::exec(config, args).await,
        "env" => env::exec(config, args).await,
        "delete" => delete::exec(config, args).await,
        "logs" => logs::exec(config, args).await,
        "sql" => sql::exec(config, args).await,
        "mcp" => mcp::exec(config, args).await,
        "rename" => dns::exec(config, args).await,
        "generate" => generate::exec(config, args).await,
        "list" => list::exec(config, args).await,
        "init" => init::exec(config, args).await.map(|_| ()),
        "build" => build::exec(config, args).await.map(drop),
        "server" => server::exec(config, paths, args).await,
        "subscribe" => subscribe::exec(config, args).await,
        "start" => return start::exec(config, paths, args).await,
        "login" => login::exec(config, args).await,
        "logout" => logout::exec(config, args).await,
        "lock" => lock::exec(config, args).await,
        "unlock" => unlock::exec(config, args).await,
        "auth" => auth::exec(config, args).await,
        "version" => return subcommands::version::exec(paths, root_dir, args).await,
        unknown => Err(anyhow::anyhow!("Invalid subcommand: {unknown}")),
    }
    .map(|()| ExitCode::SUCCESS)
}

/// An error type indicating that the process should exit silently with the
/// given `ExitCode`.
#[cfg(feature = "commands")]
#[derive(thiserror::Error, Debug)]
#[error("exit with {0:?}")]
pub struct ExitWithCode(pub ExitCode);

#[cfg(feature = "commands")]
impl ExitWithCode {
    /// Basic unsuccessful termination.
    pub const FAILURE: Self = ExitWithCode(ExitCode::FAILURE);
}
