mod account;
mod amount;
mod cli;
mod processor;

use std::{fs::File, io::stdout};

use clap::Parser;

#[derive(Parser)]
struct Args {
    #[clap(value_parser)]
    file_path: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::try_parse()?;
    let file = File::open(args.file_path)?;
    let _ = cli::run(file, stdout()).await?;
    Ok(())
}
