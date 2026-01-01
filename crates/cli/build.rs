#[cfg(feature = "commands")]
#[path = "build/commands.rs"]
mod commands;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "commands")]
    commands::run()?;
    Ok(())
}
