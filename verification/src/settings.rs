use crate::consts::DEFAULT_COMPILER_LIST;
use anyhow::anyhow;
use config::{Config, File};
use cron::Schedule;
use serde::{de::IgnoredAny, Deserialize};
use std::{net::SocketAddr, num::NonZeroUsize, path::PathBuf, str::FromStr};
use url::Url;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub server: ServerSettings,
    pub solidity: SoliditySettings,
    pub sourcify: SourcifySettings,
    pub metrics: MetricsSettings,

    #[serde(rename = "config")]
    pub config_path: IgnoredAny,
}

impl PartialEq for Settings {
    fn eq(&self, other: &Self) -> bool {
        self.server == other.server
            && self.solidity == other.solidity
            && self.sourcify == other.sourcify
            && self.metrics == other.metrics
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerSettings {
    pub addr: SocketAddr,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            addr: SocketAddr::from_str("0.0.0.0:8043").expect("should be valid url"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoliditySettings {
    pub enabled: bool,
    pub compilers_dir: PathBuf,
    #[serde(with = "serde_with::rust::display_fromstr")]
    pub refresh_versions_schedule: Schedule,
    pub fetcher: FetcherSettings,
}

impl Default for SoliditySettings {
    fn default() -> Self {
        let mut default_dir = std::env::temp_dir();
        default_dir.push("compilers");
        Self {
            enabled: true,
            compilers_dir: default_dir,
            refresh_versions_schedule: Schedule::from_str("0 0 * * * * *").unwrap(), // every hour
            fetcher: Default::default(),
        }
    }
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum FetcherSettings {
    List(ListFetcherSettings),
    S3(S3FetcherSettings),
}

impl Default for FetcherSettings {
    fn default() -> Self {
        Self::List(Default::default())
    }
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct ListFetcherSettings {
    pub list_url: Url,
}

impl Default for ListFetcherSettings {
    fn default() -> Self {
        Self {
            list_url: Url::try_from(DEFAULT_COMPILER_LIST).expect("valid url"),
        }
    }
}

#[derive(Deserialize, Default, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct S3FetcherSettings {
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub bucket: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SourcifySettings {
    pub enabled: bool,
    pub api_url: Url,
    /// Number of attempts the server makes to Sourcify API.
    /// Should be at least one. Set to `3` by default.
    pub verification_attempts: NonZeroUsize,
    pub request_timeout: u64,
}

impl Default for SourcifySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            api_url: Url::try_from("https://sourcify.dev/server/").expect("valid url"),
            verification_attempts: NonZeroUsize::new(3).expect("Is not zero"),
            request_timeout: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetricsSettings {
    pub endpoint: String,
    pub addr: SocketAddr,
}

impl Default for MetricsSettings {
    fn default() -> Self {
        Self {
            endpoint: "/metrics".to_string(),
            addr: SocketAddr::from_str("0.0.0.0:6060").expect("should be valid url"),
        }
    }
}

impl Settings {
    pub fn new() -> anyhow::Result<Self> {
        let config_path = std::env::var("VERIFICATION__CONFIG");

        let mut builder = Config::builder();
        if let Ok(config_path) = config_path {
            builder = builder.add_source(File::with_name(&config_path));
        };
        // Use `__` so that it would be possible to address keys with underscores in names (e.g. `access_key`)
        builder =
            builder.add_source(config::Environment::with_prefix("VERIFICATION").separator("__"));

        let settings: Settings = builder.build()?.try_deserialize()?;

        settings.validate()?;

        Ok(settings)
    }

    fn validate(&self) -> anyhow::Result<()> {
        // Validate s3 fetcher
        if let FetcherSettings::S3(settings) = &self.solidity.fetcher {
            if settings.region.is_none() && settings.endpoint.is_none() {
                return Err(anyhow!("for s3 fetcher settings at least one of `region` or `endpoint` should be defined"));
            }
        };

        Ok(())
    }
}
