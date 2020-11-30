use std::{
    env,
    fs::{write, File},
    io::{self, Read},
    path::PathBuf,
};

use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, Weekday};
use log::{debug, error, info, warn};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use surf::{
    middleware::{Logger, Middleware, Next},
    Client, Request, Response, Url,
};

use crate::{API_KEY_ENV_NAME, CONFIG_FILE_PATH_ENV_VAR, DEFAULT_CONFIG_PATH, SLACK_API_URL, SlackRError, api::{self, SlackAPIResultResponse, SlackApiError}, convert_date_string_to_local};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BotConfig {
    /// The list of memmbers ids that can be selected. Adds all of the channel when added to a channel.
    members: Vec<String>,
    /// The channel on which this bot will post. Single channel per config. 
    /// You may  have a different config file for different channels, although this behaviour is untested yet.
    channel: String,
    /// As input only accepts dates, this is the time that will be applied to the input date.
    target_time: NaiveTime,
    /// Possible offset for the actual time at which the message will be posted, to give some leeway for the joke to be prepared. 
    /// Set to 0 to schedule at the target time.
    target_time_schedule_offset: i64,
    /// Slack API token for the bot.
    token: Option<String>,
}

impl Default for BotConfig {
    fn default() -> Self {
        BotConfig {
            members: Vec::new(),
            channel: "channel".to_string(),
            target_time: NaiveTime::from_hms(11, 15, 0),
            // this is the 
            target_time_schedule_offset: Duration::hours(23).num_seconds(),
            token: None,
        }
    }
}

impl BotConfig {
    fn new() -> BotConfig {
        info!("Reading config path from {} env var", CONFIG_FILE_PATH_ENV_VAR);
        let path = Self::get_config_path();
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
    
    /// Gets the path to save/read the config file from either the environnment variable if set, or Default
    fn get_config_path() -> PathBuf {
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

    // Writes config to file.
    fn to_file(self) -> Result<(), SlackRError> {
        let path = Self::get_config_path();
        debug!("Writing to path {:?}", path);
        let json = serde_json::to_string_pretty(&self).expect("Couldn't serialize BotConfig");
        debug!("Serialized BotConfig as {}", json);
        write(&path, json.as_bytes()).map_err(|e| {
            error!("Couldn't write to file at the path {:?}. Error: {}", path, e);
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
                (_, _,true) => { debug!("Didn't find token, but you get a free pass"); String::from("") },
                (_, _, _) => panic!("Token was not set. You can set it with the {} environnment variable, or using the `add token` command", API_KEY_ENV_NAME)
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

    
    pub fn save(self) {
        match self.config.to_file() {
            Ok(_) => info!("Successfully saved file."),
            Err(_) => warn!("Couldnt' save file")
        }
    }

    pub async fn joke(self, input_date_arg: Option<&str>) {
        info!("Processing joke command");
        let today_with_target_time = Local::today().and_time(self.config.target_time).unwrap();
        
        let target_date: DateTime<Local> = {
            let unfiltered = match input_date_arg {
                Some(input_date_str) => {
                    debug!("Date {} was input", input_date_str);
                    // let input_plus_time = format!("{} {} {}", input_date_str, today_with_target_time.time(), today_with_target_time.offset());

                    // Unwrapping is okay as it's been validated already by clap's matcher
                    convert_date_string_to_local(&input_date_str, &today_with_target_time).unwrap()
                }
                None => {
                    debug!("No date was input. Getting tomorrow.");
                    today_with_target_time.date().succ().and_time(self.config.target_time).expect("Tomorrow may not exist. That's dark man.")
                }
            };
            match unfiltered.date().weekday() {
                // weekend dates shifted to next monday
                Weekday::Sat => unfiltered + Duration::days(2),
                Weekday::Sun => unfiltered + Duration::days(1),
                _ => unfiltered
            }
        };
        info!("Target datetime: {}", target_date);
        
        let schedule_date = {
            let unfiltered = target_date - Duration::seconds(self.config.target_time_schedule_offset);
            match unfiltered.date().weekday() {
                Weekday::Sun => {
                    debug!("Joke for a monday, scheduled to sunday, shifting to friday before");
                    unfiltered - Duration::days(2)
                },
                _ => unfiltered
            }
        };
        info!("Message schedule datetime: {}", schedule_date);
        
        if schedule_date <= Local::now() {
            error!("Too late to post for {}!", schedule_date);
            return;
        }
        
        debug!("Checking it isn't already scheduled for channel...");
        if self.date_already_been_scheduled(schedule_date, &self.config.channel).await {
            error!("This date has already been scheduled. Check with `scheduled` command");
            return;
        };
        debug!("Nothing scheduled on {}, continuing", schedule_date);
        
        let member = match self.select_random_member() {
            Some(m) => {
                info!("Selected member {}", m);
                m
            },
            None => {
                error!("No member could be selected!");
                return;
            }
        };
        
        let text = format!(
            "<@{}> will be in charge of a joke on {}!",
            member,
            target_date.naive_local().date()
        );

        let request = api::ScheduleMessageRequest::new(
            &self.config.channel, 
            schedule_date.timestamp(), 
            text);
        let response = api::schedule_message(&self.client, &request).await;
        println!("Successfully assigned member {} for a joke on {}. Message will be posted at {}. Schedule ID: {}", 
            member,
            target_date,
            response.post_at,
            response.scheduled_message_id
        );
    }

    fn select_random_member(&self) -> Option<String> {
        debug!("Selecting member");
        let mut rng = rand::thread_rng();
        match self.config.members.choose(&mut rng) {
            Some(member) => Some(member.to_owned()),
            None => None
        }
    }

    pub async fn config(
        mut self,
        members_opt: Option<Vec<String>>,
        channel_opt: Option<&str>,
        token_opt: Option<&str>,
        target_time_opt: Option<&str>,
    ) {
        info!("Processing config command");
        // let mut build_config = self.config;
        debug!("Parsing given config arguments");
        if let Some(members) = members_opt {
            debug!("Got members {:?}", members);
            for email in members {
                self.add_member_from_email(&email).await;
            };
        };
        if let Some(channel) = channel_opt {
            info!("Channel: {}", channel); 
            self.add_channel(channel).await;
        };

        if let Some(token) = token_opt { 
            info!("Token: {}", token); 
            self.add_token(token)
        };

        if let Some(target_time) = target_time_opt {
            info!("Target time: {}", target_time);
            self.add_target_time(target_time);
        };
        println!("{:?}", self.config);
        let path = BotConfig::get_config_path();
        println!("Save to file at {:?}? y/n", path);
        if yes_no_input() {
            self.save();
        }
    }

    /// Takes a user email as input, fetches its ID and adds its ID to the config members (with confirmation for matching user)
    pub async fn add_member_from_email(&mut self, email: &str) {
        info!("Processing add member command");
        let request = api::UserLookupRequest{ email: email.to_string() };
        let member_object = api::call_endpoint(api::UserLookupByEmailEndpoint, &request, &self.client).await;
        match member_object {
            SlackAPIResultResponse::Ok(response) => {
                let name = response.user.profile.display_name.or(Some(response.user.name)).unwrap();
                println!("Found user {}. Is it who you want, save its ID {} in config? y/n", name, response.user.id);
                if yes_no_input() {
                    self.config.members.push(response.user.id);
                }
            },
            SlackAPIResultResponse::Err(slack_err) => {
                match slack_err.error {
                    api::SlackApiError::users_not_found => error!("User email was not found, or the bot doesn't have access to it."),
                    api::SlackApiError::missing_scope => error!("Usage of lookup by email requires the Slack `users:read.email` scope. Please verify bot permissions."),
                    _ => error!("{:?}", slack_err.error)
                };
            }
        }
    }

    pub async fn add_channel(&mut self, channel: &str) {
        // shadowing to string
        let channel = channel.to_string();

        if self.config.channel == channel {
            error!("Channel {} is already the current channel", channel);
        };

        let request = api::JoinConversationRequest { channel: channel.clone() };
        let join_channel_response = api::call_endpoint(api::JoinConversationEndpoint, &request, &self.client).await;
        match join_channel_response {
            SlackAPIResultResponse::Ok(response) => {
                if response.warning == Some("already_in_channel".to_string()) {
                    warn!("Was already in channel.");
                } else {
                    info!("Successfully joined channel.");
                }
            },
            SlackAPIResultResponse::Err(err) => {
                error!("Couldn't join channel. Error: {:?}. Aborting.", err.error);
                return;
            }
        };

        let members = match api::list_members_for_channel(&self.client, &channel).await {
            Ok(m) => m,
            Err(err) => {
                match err {
                    SlackApiError::invalid_channel 
                    | SlackApiError::channel_not_found => error!("The channel {} is invalid", channel),
                    _ => error!("Slack error: {:?}.", err)
                }
                warn!("Adding empty members list");
                vec![]
            }
        };
        self.config.members.extend(members);
        self.config.channel = channel;
    }

    pub fn add_token(&mut self, token: &str) {
        self.config.token = Some(token.to_string());
    }

    pub fn add_target_time(&mut self, target_time: &str) {
        self.config.target_time = NaiveTime::parse_from_str(target_time, "%H:%M:%S")
            .expect("Unable to parse target time");
    }        

    pub fn add_offset_time(&mut self, offset: &str) {
        self.config.target_time_schedule_offset = offset.parse::<i64>()
            .expect("Unable to parse offset time");
    }

    pub async fn check_scheduled_messages(self) {
        let messages = api::list_scheduled_messages(&self.client, &self.config.channel).await;
        info!("Printing {} scheduled messages for channel {}", messages.len(), self.config.channel);
        for mess in messages {
            println!("{}", mess);
        }
    }

    async fn date_already_been_scheduled(&self, date: DateTime<Local>, channel: &str) -> bool {
        let messages = api::list_scheduled_messages(&self.client, channel).await;
        if messages.is_empty() { return false };
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
        req.insert_header(surf::http::headers::CONTENT_TYPE, format!("{}; charset=utf-8", surf::http::mime::JSON));
        let res = next.run(req, client).await?;
        Ok(res)
    }
}

fn yes_no_input() -> bool {
    let mut buff = String::new();
    match std::io::stdin().read_line(&mut buff) {
        Ok(_bytes) => {
            if buff.to_ascii_lowercase().trim() == "y".to_string() {
                true
            } else { false }
        }
        Err(_err) => { false },
    }
}