use super::*;
use crate::{SlackRError, CONFIG_FILE_PATH_ENV_VAR, DEFAULT_CONFIG_PATH};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{write, File},
    io::{self, Read},
    path::PathBuf,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    /// The list of members ids that can be selected. Adds all of the channel when added to a channel.
    pub members: Vec<String>,
    /// Members that have been selected.
    pub selected: Vec<String>,
    /// The channel on which this bot will post. Single channel per config.
    /// You may  have a different config file for different channels, although this behaviour is untested yet.
    pub channel: String,
    /// As input only accepts dates, this is the time that will be applied to the input date.
    pub target_time: NaiveTime,
    /// Possible offset for the actual time at which the message will be posted, to give some leeway for the joke to be prepared.
    /// How many days in avance to schedule the post, relative to the target time.
    pub advance_days: i64,
    /// On the day from `advance_days`, post at this time.
    pub post_time: NaiveTime,
    /// Delay for "instant" schedules, such as the reroll. Defaults to 45s.
    pub instant_delay: i64,
    /// Slack API token for the bot.
    pub token: Option<String>,
    // Bot self Id, acquired as a check for the token, and making sure it never adds itself as member.
    pub id: String,
}

impl Default for BotConfig {
    fn default() -> Self {
        BotConfig {
            members: Vec::new(),
            selected: Vec::new(),
            channel: String::new(),
            target_time: NaiveTime::from_hms(11, 30, 0),
            post_time: NaiveTime::from_hms(11, 30, 0),
            advance_days: 1,
            instant_delay: 45,
            token: None,
            id: String::new(),
        }
    }
}

impl BotConfig {
    pub fn new() -> BotConfig {
        info!(
            "Reading config path from {} env var",
            CONFIG_FILE_PATH_ENV_VAR
        );
        let path = Self::get_config_path();
        debug!("Got path");
        match File::open(&path) {
            Ok(file) => {
                debug!("Config file already exists at the path. Setting up from it.");
                BotConfig::from_file(file).unwrap_or_default()
            }
            Err(err) => {
                debug!("Error opening file: {}", err);
                // try to output a helpful message
                match err.kind() {
                    io::ErrorKind::NotFound => info!("Config file doesn't exist yet. Create manually and set up env var, or run `slack-r config` to create it"),
                    io::ErrorKind::PermissionDenied => {
                        if path.file_name().is_none() {
                            error!("The path in {} must contain the file name! Using default config instead.", CONFIG_FILE_PATH_ENV_VAR);
                        } else {
                            error!("Permission denied when trying to open file at {}. Using default config instead.", CONFIG_FILE_PATH_ENV_VAR);
                        }
                    },
                    _ => warn!("Error opening file at {}. {}", CONFIG_FILE_PATH_ENV_VAR, err)
                }
                debug!("Creating default BotConfig");
                BotConfig::default()
            }
        }
    }

    /// Gets the path to save/read the config file from either the environnment variable if set, or Default
    pub fn get_config_path() -> PathBuf {
        match env::var(CONFIG_FILE_PATH_ENV_VAR) {
            Ok(path_string) => {
                info!(
                    "Env var {} set. Using path {}.",
                    CONFIG_FILE_PATH_ENV_VAR, path_string
                );
                PathBuf::from(path_string)
            }
            Err(_e) => {
                info!(
                    "Env var {} not set. Using default path {}.",
                    CONFIG_FILE_PATH_ENV_VAR, DEFAULT_CONFIG_PATH
                );
                PathBuf::from(DEFAULT_CONFIG_PATH)
            }
        }
    }

    /// Will read a given file handle, expected to contain the config in JSON format, and try to construct the config from it.
    /// Errors will be ignored and simply ignore the file and return the default values, with warnings.
    fn from_file(mut file: File) -> Result<Self, SlackRError> {
        debug!("Reading file content...");
        let mut buf = String::new();
        match file.read_to_string(&mut buf) {
            Ok(bytes_read) => debug!("Read {} bytes from the file.", bytes_read),
            Err(e) => {
                error!(
                    "Error reading file: {}. Using default config values instead",
                    e
                );
                return Err(SlackRError::CorruptedConfig);
            }
        };

        let config = serde_json::from_str::<BotConfig>(&buf).map_err(|err| {
            error!("Failed parsing config file. {}", err);
            SlackRError::CorruptedConfig
        })?;
        info!("Successfully read config from file");
        Ok(config)
    }

    // Writes config to file.
    pub fn to_file(&self) -> Result<(), SlackRError> {
        let path = Self::get_config_path();
        debug!("Writing to path {:?}", path);
        let json = serde_json::to_string_pretty(&self).expect("Couldn't serialize BotConfig");
        debug!("Serialized BotConfig as {}", json);
        write(&path, json.as_bytes()).map_err(|e| {
            error!(
                "Couldn't write to file at the path {:?}. Error: {}",
                path, e
            );
            SlackRError::WriteConfig
        })?;
        info!("Saved config to file {:?}", path);
        Ok(())
    }
}
