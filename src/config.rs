use std::{fmt, fs, path::Path, path::PathBuf};

use serde::Deserialize;
use thiserror::Error;

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 4002;
pub const DEFAULT_SERIAL_BAUD_RATE: u32 = 115_200;
pub const DEFAULT_SERIAL_DELIMITER: &str = "\r\n";
pub const LOCAL_CONFIG_FILE_NAME: &str = "serialport-api.toml";

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct FileConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub serial: SerialConfig,
    #[serde(default)]
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct StorageConfig {
    pub preset_db: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct SerialConfig {
    pub default_port: Option<String>,
    pub default_baud_rate: Option<u32>,
    pub default_delimiter: Option<String>,
    pub real_serial: Option<bool>,
    pub mock_device: Option<bool>,
    pub mock_script: Option<PathBuf>,
}

#[derive(Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    ParseFile {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid SERIALPORT_API_PORT value {value:?}: {source}")]
    InvalidEnvPort {
        value: String,
        #[source]
        source: std::num::ParseIntError,
    },
    #[error("--real-serial cannot be combined with --mock-device or --mock-script")]
    IncompatibleModes,
}

impl fmt::Debug for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl FileConfig {
    pub fn from_toml_str(input: &str) -> Result<Self, ConfigError> {
        toml::from_str(input).map_err(ConfigError::Parse)
    }
}

pub fn load_explicit_config(path: impl AsRef<Path>) -> Result<FileConfig, ConfigError> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&contents).map_err(|source| ConfigError::ParseFile {
        path: path.to_path_buf(),
        source,
    })
}

pub fn load_discovered_config(cwd: impl AsRef<Path>) -> Result<FileConfig, ConfigError> {
    let path = cwd.as_ref().join(LOCAL_CONFIG_FILE_NAME);
    if path.exists() {
        load_explicit_config(path)
    } else {
        Ok(FileConfig::default())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliServeOverrides {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub mock_device: bool,
    pub mock_script: Option<PathBuf>,
    pub real_serial: bool,
    pub preset_db: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvServeConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
}

impl EnvServeConfig {
    pub fn from_current_env() -> Result<Self, ConfigError> {
        Self::from_vars(
            std::env::var("SERIALPORT_API_HOST").ok(),
            std::env::var("SERIALPORT_API_PORT").ok(),
        )
    }

    pub fn from_vars(host: Option<String>, port: Option<String>) -> Result<Self, ConfigError> {
        let port = match port {
            Some(value) => Some(
                value
                    .parse()
                    .map_err(|source| ConfigError::InvalidEnvPort { value, source })?,
            ),
            None => None,
        };

        Ok(Self { host, port })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialDefaults {
    pub default_port: Option<String>,
    pub default_baud_rate: u32,
    pub default_delimiter: String,
}

impl Default for SerialDefaults {
    fn default() -> Self {
        Self {
            default_port: None,
            default_baud_rate: DEFAULT_SERIAL_BAUD_RATE,
            default_delimiter: DEFAULT_SERIAL_DELIMITER.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedServeConfig {
    pub host: String,
    pub port: u16,
    pub mock_device: bool,
    pub mock_script: Option<PathBuf>,
    pub real_serial: bool,
    pub serial_defaults: SerialDefaults,
    pub preset_db: Option<PathBuf>,
}

impl fmt::Display for ResolvedServeConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.host, self.port)
    }
}

pub fn resolve_serve_config(
    cli: CliServeOverrides,
    env: EnvServeConfig,
    file: FileConfig,
) -> Result<ResolvedServeConfig, ConfigError> {
    let mock_script = cli.mock_script.or_else(|| file.serial.mock_script.clone());
    let mock_device =
        cli.mock_device || file.serial.mock_device.unwrap_or(false) || mock_script.is_some();
    let real_serial = cli.real_serial || file.serial.real_serial.unwrap_or(false);

    let resolved = ResolvedServeConfig {
        host: cli
            .host
            .or(env.host)
            .or(file.server.host)
            .unwrap_or_else(|| DEFAULT_HOST.to_string()),
        port: cli
            .port
            .or(env.port)
            .or(file.server.port)
            .unwrap_or(DEFAULT_PORT),
        mock_device,
        mock_script,
        real_serial,
        preset_db: cli.preset_db.or(file.storage.preset_db),
        serial_defaults: SerialDefaults {
            default_port: file.serial.default_port,
            default_baud_rate: file
                .serial
                .default_baud_rate
                .unwrap_or(DEFAULT_SERIAL_BAUD_RATE),
            default_delimiter: file
                .serial
                .default_delimiter
                .unwrap_or_else(|| DEFAULT_SERIAL_DELIMITER.to_string()),
        },
    };

    validate_resolved_serve_config(&resolved)?;
    Ok(resolved)
}

pub fn validate_resolved_serve_config(config: &ResolvedServeConfig) -> Result<(), ConfigError> {
    if config.real_serial && (config.mock_device || config.mock_script.is_some()) {
        Err(ConfigError::IncompatibleModes)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn parses_server_and_serial_config_from_toml() {
        let config = FileConfig::from_toml_str(
            r#"
[server]
host = "0.0.0.0"
port = 5000

[serial]
default_port = "/dev/ttyUSB0"
default_baud_rate = 57600
default_delimiter = "\n"
real_serial = true
mock_device = false
mock_script = "./mock-responses.json"
"#,
        )
        .unwrap();

        assert_eq!(config.server.host.as_deref(), Some("0.0.0.0"));
        assert_eq!(config.server.port, Some(5000));
        assert_eq!(config.serial.default_port.as_deref(), Some("/dev/ttyUSB0"));
        assert_eq!(config.serial.default_baud_rate, Some(57_600));
        assert_eq!(config.serial.default_delimiter.as_deref(), Some("\n"));
        assert_eq!(config.serial.real_serial, Some(true));
        assert_eq!(config.serial.mock_device, Some(false));
        assert_eq!(
            config.serial.mock_script,
            Some(PathBuf::from("./mock-responses.json"))
        );
    }

    #[test]
    fn omitted_sections_default_to_none_values() {
        let config = FileConfig::from_toml_str("").unwrap();

        assert_eq!(config, FileConfig::default());
    }

    #[test]
    fn explicit_existing_config_loads_successfully() {
        let temp_dir = unique_temp_dir("explicit_existing_config_loads_successfully");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let path = temp_dir.join("config.toml");
        std::fs::write(&path, "[server]\nport = 4012\n").unwrap();

        let config = load_explicit_config(&path).unwrap();

        assert_eq!(config.server.port, Some(4012));
        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn explicit_missing_config_errors() {
        let path = unique_temp_dir("explicit_missing_config_errors").join("missing.toml");

        let error = load_explicit_config(&path).unwrap_err();

        assert!(matches!(error, ConfigError::Read { .. }));
    }

    #[test]
    fn auto_discovery_missing_config_returns_defaults() {
        let temp_dir = unique_temp_dir("auto_discovery_missing_config_returns_defaults");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let config = load_discovered_config(&temp_dir).unwrap();

        assert_eq!(config, FileConfig::default());
        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn auto_discovery_loads_local_project_config_when_present() {
        let temp_dir = unique_temp_dir("auto_discovery_loads_local_project_config_when_present");
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(
            temp_dir.join(LOCAL_CONFIG_FILE_NAME),
            "[server]\nhost = \"127.0.0.2\"\n",
        )
        .unwrap();

        let config = load_discovered_config(&temp_dir).unwrap();

        assert_eq!(config.server.host.as_deref(), Some("127.0.0.2"));
        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn config_values_fill_built_in_defaults() {
        let file = FileConfig::from_toml_str(
            r#"
[server]
host = "0.0.0.0"
port = 5000
[serial]
default_port = "/dev/ttyUSB0"
default_baud_rate = 9600
default_delimiter = "\n"
mock_device = true
"#,
        )
        .unwrap();

        let resolved = resolve_serve_config(
            CliServeOverrides::default(),
            EnvServeConfig::default(),
            file,
        )
        .unwrap();

        assert_eq!(resolved.host, "0.0.0.0");
        assert_eq!(resolved.port, 5000);
        assert!(resolved.mock_device);
        assert_eq!(
            resolved.serial_defaults.default_port.as_deref(),
            Some("/dev/ttyUSB0")
        );
        assert_eq!(resolved.serial_defaults.default_baud_rate, 9600);
        assert_eq!(resolved.serial_defaults.default_delimiter, "\n");
    }

    #[test]
    fn env_overrides_config_for_host_and_port() {
        let file =
            FileConfig::from_toml_str("[server]\nhost = \"0.0.0.0\"\nport = 5000\n").unwrap();
        let env =
            EnvServeConfig::from_vars(Some("127.0.0.3".to_string()), Some("6000".to_string()))
                .unwrap();

        let resolved = resolve_serve_config(CliServeOverrides::default(), env, file).unwrap();

        assert_eq!(resolved.host, "127.0.0.3");
        assert_eq!(resolved.port, 6000);
    }

    #[test]
    fn cli_overrides_env_and_config_for_host_and_port() {
        let file =
            FileConfig::from_toml_str("[server]\nhost = \"0.0.0.0\"\nport = 5000\n").unwrap();
        let env =
            EnvServeConfig::from_vars(Some("127.0.0.3".to_string()), Some("6000".to_string()))
                .unwrap();
        let cli = CliServeOverrides {
            host: Some("127.0.0.4".to_string()),
            port: Some(7000),
            ..CliServeOverrides::default()
        };

        let resolved = resolve_serve_config(cli, env, file).unwrap();

        assert_eq!(resolved.host, "127.0.0.4");
        assert_eq!(resolved.port, 7000);
    }

    #[test]
    fn mock_script_implies_mock_device() {
        let file =
            FileConfig::from_toml_str("[serial]\nmock_script = \"./script.json\"\n").unwrap();

        let resolved = resolve_serve_config(
            CliServeOverrides::default(),
            EnvServeConfig::default(),
            file,
        )
        .unwrap();

        assert!(resolved.mock_device);
        assert_eq!(resolved.mock_script, Some(PathBuf::from("./script.json")));
    }

    #[test]
    fn real_serial_with_mock_device_or_script_is_rejected_after_resolution() {
        let file = FileConfig::from_toml_str("[serial]\nreal_serial = true\nmock_device = true\n")
            .unwrap();

        let error = resolve_serve_config(
            CliServeOverrides::default(),
            EnvServeConfig::default(),
            file,
        )
        .unwrap_err();

        assert!(matches!(error, ConfigError::IncompatibleModes));
    }

    #[test]
    fn built_in_defaults_are_preserved_without_config_cli_or_env() {
        let resolved = resolve_serve_config(
            CliServeOverrides::default(),
            EnvServeConfig::default(),
            FileConfig::default(),
        )
        .unwrap();

        assert_eq!(resolved.host, DEFAULT_HOST);
        assert_eq!(resolved.port, DEFAULT_PORT);
        assert!(!resolved.mock_device);
        assert!(resolved.mock_script.is_none());
        assert!(!resolved.real_serial);
        assert_eq!(resolved.serial_defaults, SerialDefaults::default());
        assert!(resolved.preset_db.is_none());
    }

    #[test]
    fn storage_config_resolves_preset_db_and_cli_override_wins() {
        let file =
            FileConfig::from_toml_str("[storage]\npreset_db = \"./from-config.db\"\n").unwrap();
        let resolved = resolve_serve_config(
            CliServeOverrides::default(),
            EnvServeConfig::default(),
            file,
        )
        .unwrap();
        assert_eq!(resolved.preset_db, Some(PathBuf::from("./from-config.db")));

        let cli = CliServeOverrides {
            preset_db: Some(PathBuf::from("./from-cli.db")),
            ..CliServeOverrides::default()
        };
        let file =
            FileConfig::from_toml_str("[storage]\npreset_db = \"./from-config.db\"\n").unwrap();
        let resolved = resolve_serve_config(cli, EnvServeConfig::default(), file).unwrap();
        assert_eq!(resolved.preset_db, Some(PathBuf::from("./from-cli.db")));
    }

    fn unique_temp_dir(test_name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "serialport-api-phase12-{test_name}-{}",
            std::process::id()
        ))
    }
}
