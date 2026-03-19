use clap::Parser;
use cli::Cli;

use crate::app::App;

mod action;
mod app;
mod aws;
mod cli;
mod components;
mod config;
mod errors;
mod logging;
mod tui;
mod resource_selector;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();
    let mut app = App::new(args.tick_rate, args.frame_rate).await?;
    app.run().await?;
    Ok(())
}
