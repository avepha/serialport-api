use std::{net::SocketAddr, path::PathBuf};

use clap::{Args, Parser, Subcommand};
use serialport_api::{
    api::routes,
    serial::{
        manager::{ConnectionManagerWithTransport, SystemPortLister},
        mock_device::{MockDeviceResponder, MockResponseScript},
        transport::MockSerialTransport,
    },
};

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

    #[arg(long)]
    mock_device: bool,

    #[arg(long)]
    mock_script: Option<PathBuf>,
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

    tracing::info!(%addr, mock_device = args.mock_device || args.mock_script.is_some(), "listening");

    let app = if args.mock_device || args.mock_script.is_some() {
        let script = match &args.mock_script {
            Some(path) => {
                let contents = std::fs::read_to_string(path)?;
                MockResponseScript::from_json_str(&contents)?
            }
            None => MockResponseScript::default(),
        };
        let manager = ConnectionManagerWithTransport::with_mock_responder(
            MockSerialTransport::default(),
            MockDeviceResponder::from_script(script),
        );
        routes::router_with_state(routes::AppState::new(SystemPortLister, manager))
    } else {
        routes::router()
    };

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serve_cli_accepts_mock_device_and_mock_script() {
        let cli = Cli::parse_from([
            "serialport-api",
            "serve",
            "--mock-device",
            "--mock-script",
            "mock-responses.json",
        ]);

        let Some(Command::Serve(args)) = cli.command else {
            panic!("expected serve command");
        };
        assert!(args.mock_device);
        assert_eq!(
            args.mock_script.unwrap(),
            std::path::PathBuf::from("mock-responses.json")
        );
    }
}
