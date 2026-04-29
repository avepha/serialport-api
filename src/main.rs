use std::net::SocketAddr;

use clap::{Args, Parser, Subcommand};
use serialport_api::api::routes;

#[derive(Debug, Parser)]
#[command(name = "serialport-api", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve(ServeArgs),
}

#[derive(Debug, Args)]
struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1", env = "SERIALPORT_API_HOST")]
    host: String,

    #[arg(long, default_value_t = 4002, env = "SERIALPORT_API_PORT")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Serve(args)) => serve(args).await?,
        None => println!("serialport-api: rewrite in progress"),
    }

    Ok(())
}

async fn serve(args: ServeArgs) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(%addr, "listening");

    axum::serve(listener, routes::router()).await?;

    Ok(())
}
