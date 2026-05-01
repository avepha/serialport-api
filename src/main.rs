use std::{net::SocketAddr, path::PathBuf};

use clap::{Args, Parser, Subcommand};
use serialport_api::{
    api::routes,
    serial::{
        manager::{ConnectionManagerWithTransport, SystemPortLister},
        mock_device::{MockDeviceResponder, MockResponseScript},
        real_transport::SystemRealSerialConnectionManager,
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

    #[arg(long)]
    real_serial: bool,
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
    validate_serve_args(&args)?;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(%addr, mock_device = args.mock_device || args.mock_script.is_some(), real_serial = args.real_serial, "listening");

    let app = if args.real_serial {
        routes::router_with_state(routes::AppState::new(
            SystemPortLister,
            SystemRealSerialConnectionManager::default(),
        ))
    } else if args.mock_device || args.mock_script.is_some() {
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

fn validate_serve_args(args: &ServeArgs) -> Result<(), String> {
    if args.real_serial && args.mock_device {
        return Err("--real-serial cannot be combined with --mock-device".to_string());
    }

    if args.real_serial && args.mock_script.is_some() {
        return Err("--real-serial cannot be combined with --mock-script".to_string());
    }

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
        assert!(!args.real_serial);
        assert_eq!(
            args.mock_script.unwrap(),
            std::path::PathBuf::from("mock-responses.json")
        );
    }

    #[test]
    fn serve_cli_accepts_real_serial_flag() {
        let cli = Cli::parse_from(["serialport-api", "serve", "--real-serial"]);

        let Some(Command::Serve(args)) = cli.command else {
            panic!("expected serve command");
        };
        assert!(args.real_serial);
        assert!(!args.mock_device);
        assert!(args.mock_script.is_none());
    }

    #[test]
    fn serve_args_reject_real_serial_with_mock_device() {
        let cli = Cli::parse_from(["serialport-api", "serve", "--real-serial", "--mock-device"]);
        let Some(Command::Serve(args)) = cli.command else {
            panic!("expected serve command");
        };

        let error = validate_serve_args(&args).unwrap_err();

        assert!(error.contains("--real-serial"));
        assert!(error.contains("--mock-device"));
    }

    #[test]
    fn serve_args_reject_real_serial_with_mock_script() {
        let cli = Cli::parse_from([
            "serialport-api",
            "serve",
            "--real-serial",
            "--mock-script",
            "mock-responses.json",
        ]);
        let Some(Command::Serve(args)) = cli.command else {
            panic!("expected serve command");
        };

        let error = validate_serve_args(&args).unwrap_err();

        assert!(error.contains("--real-serial"));
        assert!(error.contains("--mock-script"));
    }
}
