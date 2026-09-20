mod cli;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    cli::Cli::parse().run()
}
