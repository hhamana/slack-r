use std::{
    collections::HashMap,
    env,
    fs::{write, File},
    io::{self, Read},
    path::PathBuf,
};

use chrono::{DateTime, Duration, Local, NaiveTime};
use log::{debug, error, info, warn};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use surf::{
    middleware::{Logger, Middleware, Next},
    Client, Request, Response, Url,
};

use crate::{
    api, convert_date_string_to_local, SlackRError, API_KEY_ENV_NAME, CONFIG_FILE_PATH_ENV_VAR,
    DEFAULT_CONFIG_PATH, SLACK_API_URL,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BotConfig {
    members: Vec<String>,
    channel: String,
    target_time: NaiveTime,
    token: Option<String>,
}

impl Default for BotConfig {
    fn default() -> Self {
        BotConfig {
            members: Vec::new(),
            channel: "channel".to_string(),
            target_time: NaiveTime::from_hms(11, 15, 0) - Duration::hours(23),
            token: None,
        }
    }
}

impl BotConfig {
    fn new() -> BotConfig {
        info!("Reading config path from {} env var", CONFIG_FILE_PATH_ENV_VAR);
        let env_value = env::var(CONFIG_FILE_PATH_ENV_VAR);
        let path = match env_value {
            Ok(value) => {
                debug!("Env var {} was set to {}", CONFIG_FILE_PATH_ENV_VAR, value);
                PathBuf::from(value)
            }
            Err(_e) => {
                debug!("Env var {} was not set, considering default path. {}", CONFIG_FILE_PATH_ENV_VAR, DEFAULT_CONFIG_PATH);
                PathBuf::from(DEFAULT_CONFIG_PATH)
            }
        };
        debug!("Got path");
        match File::open(&path) {
            Ok(file) => {
                debug!("Config file already exists at the path. Setting up from it.");
                BotConfig::from_file(file).unwrap_or_default()
            },
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

    /// Will read a given file handle, expected to contain the config in JSON format, and try to construct the config from it.
    /// Errors will be ignored and simply ignore the file and return the default values, with warnings.
    fn from_file(mut file: File) -> Result<Self, SlackRError> {
        debug!("Reading file content...");
        let mut buf = String::new();
        match file.read_to_string(&mut buf) {
            Ok(bytes_read) => debug!("Read {} bytes from the file.", bytes_read),
            Err(e) => {
                warn!("Error reading file: {}. Using default config values instead", e);
                return Err(SlackRError);
            }
        };

        let config = serde_json::from_str::<BotConfig>(&buf).map_err(|err| {
            warn!("Failed parsing config file. {}", err);
            SlackRError
        })?;
        info!("Successfully read config from file");
        Ok(config)
    }

    fn to_file(self, path: PathBuf) -> Result<(), SlackRError> {
        debug!("Writing to path {:?}", path);
        let json = serde_json::to_string(&self).expect("Couldn't serialize BotConfig");
        debug!("Serialized BotConfig as {}", json);
        write(&path, json.as_bytes()).map_err(|e| {
            error!(
                "Couldn't write to file at the path {:?}. Error: {}",
                path, e
            );
            SlackRError
        })?;
        info!("Saved config to file {:?}", path);
        Ok(())
    }
}
pub struct SlackBot {
    client: Client,
    config: BotConfig,
}

impl SlackBot {
    pub fn new(no_panic: bool) -> Self {
        debug!("Creating bot");

        let config = BotConfig::new();
        debug!("Created config");

        debug!("Looking for API token...");
        // Force crash if the api_key env var is not set right here. This is not an accident.
        let token: String = match (
            std::env::var(API_KEY_ENV_NAME),
            &config.token,
            no_panic) {
                // 3-way pattern matching to allow for many ways to get the env var.
                // Isn't this beautiful?
                (Ok(var), _, _) => { debug!("Found token in {}", API_KEY_ENV_NAME); var},
                (_, Some(var), _ ) => { debug!("Found token in config"); var.clone()},
                (_, _,true) => { debug!("Didn't find token, but you get a free pass"); String::from("NO TOKEN INPUT") },
                (_, _, _) => panic!("Token was not set. You can set it with the {} environnment variable, or using the `config` command", API_KEY_ENV_NAME)
        };

        debug!("Creating Internet client");
        let headers = HeadersMiddleware { token };
        let mut client = Client::new()
            .with(Logger::new())
            .with(headers);
        client.set_base_url(Url::parse(SLACK_API_URL).unwrap());
        debug!("Bot setup complete");
        SlackBot { client, config }
    }

    async fn select_random_member(&self) -> Result<String, SlackRError> {
        debug!("Selecting member");
        let mut rng = rand::thread_rng();

        debug!("Getting list of members on channel");
        let request = api::ListMembersRequestParams::new(&self.config.channel);
        let remote_members = api::call_endpoint(api::ListMembersEndpoint, &request, &self.client).await;
        // let member = self.config.members.choose(&mut rng)
        //     .ok_or_else(|| { error!("No member to pick from."); SlackRError})?
        //     .to_owned();
        let member = remote_members
            .members
            .choose(&mut rng)
            .expect("No Member to pick from.");
        info!("Member picked randomly: {}", member);
        Ok(member.clone())
    }

    pub async fn joke(self, input_date_arg: Option<&str>) {
        info!("Processing joke command");
        let today = Local::now();

        let input_date: DateTime<Local> = match input_date_arg {
            Some(input_date) => {
                debug!("Date {} was input", input_date);
                // Unwrapping is okay as it's been validated already by clap's matcher
                convert_date_string_to_local(input_date, &today).unwrap()
            }
            None => {
                debug!("No date was input. Calculating next business day");
                today
                    .date()
                    .succ()
                    .and_time(today.time())
                    .expect("Somehow this time cannot exist")
                // target
            }
        };

        if self.date_already_been_scheduled(input_date).await {
            error!("This date has already been scheduled");
            return;
        };

        debug!("Using date: {}", input_date);
        let member = match self.select_random_member().await {
            Ok(member) => member,
            Err(_err) => {
                return;
            }
        };
        let text = format!(
            "<@{}> will be in charge of a joke on {}!",
            member,
            input_date.naive_local().date()
        );

        let request =
            api::ScheduleMessageRequest::new(&self.config.channel, input_date.timestamp(), text);
        let response = api::schedule_message(&self.client, &request).await;
        warn!("Response: {}", response);

        debug!("Done for now I guess");
    }

    pub async fn config(
        self,
        members_opt: Option<Vec<String>>,
        channel_opt: Option<&str>,
        token_opt: Option<&str>,
        target_time_opt: Option<&str>,
    ) {
        info!("Processing config command");
        let path = match env::var(CONFIG_FILE_PATH_ENV_VAR) {
            Ok(path_string) => {
                info!(
                    "Env var {} set. Writing to path {}.",
                    CONFIG_FILE_PATH_ENV_VAR, path_string
                );
                PathBuf::from(path_string)
            }
            Err(_e) => {
                info!(
                    "Env var {} not set. Writing to default path {}.",
                    CONFIG_FILE_PATH_ENV_VAR, DEFAULT_CONFIG_PATH
                );
                PathBuf::from(DEFAULT_CONFIG_PATH)
            }
        };
        let mut build_config = self.config;
        debug!("Parsing given config arguments");
        if let Some(members) = members_opt { info!("Member: {:?}", members); build_config.members = members };
        if let Some(channel) = channel_opt { info!("Channel: {}", channel); build_config.channel = channel.to_string() };
        if let Some(token) = token_opt { info!("Token: {}", token); build_config.token = Some(token.to_string()) };
        if let Some(target_time) = target_time_opt { 
            info!("Target time: {}", target_time);
            build_config.target_time = NaiveTime::parse_from_str(target_time, "%H:%M:%S")
                .expect("Unable to parse time again");
        };
        println!("{:?}", build_config);
        println!("Save to file at {:?}? y/n", path);
        let mut buff = String::new();
        match std::io::stdin().read_line(&mut buff) {
            Ok(_bytes) => {
                if buff.to_ascii_lowercase().trim() == "y".to_string() {
                    build_config.to_file(path).expect("Couldn't write the file");
                }
            }
            Err(err) => debug!("Failed getting stdin. {}", err),
        }
    }

    pub async fn check_scheduled_messages(self) {
        let messages = api::list_scheduled_messages(&self.client).await;
        for mess in messages {
            println!("{}", mess);
        }
    }

    async fn date_already_been_scheduled(&self, date: DateTime<Local>) -> bool {
        let messages = api::list_scheduled_messages(&self.client).await;
        messages.iter().all(|mess| mess.date() == date.date())
    }
}

struct HeadersMiddleware {
    token: String,
}

#[surf::utils::async_trait]
impl Middleware for HeadersMiddleware {
    async fn handle(
        &self,
        mut req: Request,
        client: Client,
        next: Next<'_>,
    ) -> Result<Response, surf::Error> {
        req.insert_header(
            surf::http::headers::AUTHORIZATION,
            format!("Bearer {}", self.token),
        );
        req.insert_header(surf::http::headers::CONTENT_TYPE, surf::http::mime::JSON);
        let res = next.run(req, client).await?;
        Ok(res)
    }
}
