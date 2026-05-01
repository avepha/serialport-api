use std::{net::SocketAddr, path::PathBuf};

use clap::{Args, Parser, Subcommand};
use serialport_api::{
    api::routes,
    config::{
        load_discovered_config, load_explicit_config, resolve_serve_config, CliServeOverrides,
        EnvServeConfig,
    },
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
    #[arg(long)]
    host: Option<String>,

    #[arg(long)]
    port: Option<u16>,

    #[arg(long)]
    config: Option<PathBuf>,

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
    let config_file = match &args.config {
        Some(path) => load_explicit_config(path)?,
        None => load_discovered_config(std::env::current_dir()?)?,
    };
    let resolved = resolve_serve_config(
        args.into(),
        EnvServeConfig::from_current_env()?,
        config_file,
    )?;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let addr: SocketAddr = resolved.to_string().parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(
        %addr,
        mock_device = resolved.mock_device,
        real_serial = resolved.real_serial,
        default_port = ?resolved.serial_defaults.default_port,
        default_baud_rate = resolved.serial_defaults.default_baud_rate,
        default_delimiter = ?resolved.serial_defaults.default_delimiter,
        "listening"
    );

    let app = if resolved.real_serial {
        routes::router_with_state(routes::AppState::new(
            SystemPortLister,
            SystemRealSerialConnectionManager::default(),
        ))
    } else if resolved.mock_device {
        let script = match &resolved.mock_script {
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

impl From<ServeArgs> for CliServeOverrides {
    fn from(args: ServeArgs) -> Self {
        Self {
            host: args.host,
            port: args.port,
            mock_device: args.mock_device,
            mock_script: args.mock_script,
            real_serial: args.real_serial,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serialport_api::config::{ConfigError, FileConfig};

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
    fn serve_cli_accepts_config_path() {
        let cli = Cli::parse_from([
            "serialport-api",
            "serve",
            "--config",
            "./serialport-api.toml",
        ]);

        let Some(Command::Serve(args)) = cli.command else {
            panic!("expected serve command");
        };
        assert_eq!(args.config, Some(PathBuf::from("./serialport-api.toml")));
    }

    #[test]
    fn serve_cli_accepts_host_and_port_without_clap_defaults() {
        let cli = Cli::parse_from([
            "serialport-api",
            "serve",
            "--host",
            "0.0.0.0",
            "--port",
            "5000",
        ]);

        let Some(Command::Serve(args)) = cli.command else {
            panic!("expected serve command");
        };
        assert_eq!(args.host.as_deref(), Some("0.0.0.0"));
        assert_eq!(args.port, Some(5000));
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

        let error = resolve_serve_config(
            args.into(),
            EnvServeConfig::default(),
            FileConfig::default(),
        )
        .unwrap_err();

        assert!(matches!(error, ConfigError::IncompatibleModes));
        assert!(error.to_string().contains("--real-serial"));
        assert!(error.to_string().contains("--mock-device"));
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

        let error = resolve_serve_config(
            args.into(),
            EnvServeConfig::default(),
            FileConfig::default(),
        )
        .unwrap_err();

        assert!(matches!(error, ConfigError::IncompatibleModes));
        assert!(error.to_string().contains("--real-serial"));
        assert!(error.to_string().contains("--mock-script"));
    }
}
