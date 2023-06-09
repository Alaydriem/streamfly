use std::path::Path;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use streamfly::serve;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Serve(ServeArgs),
}

#[derive(Args)]
struct ServeArgs {
    #[arg(long, default_value_t = String::from("127.0.0.1:1318"))]
    addr: String,

    #[arg(long, default_value_t = String::from("./certs/cert.pem"))]
    cert: String,

    #[arg(long, default_value_t = String::from("./certs/key.pem"))]
    key: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Serve(args) => {
            serve(&args.addr, Path::new(&args.cert), Path::new(&args.key)).await?;
        }
    }

    Ok(())
}
