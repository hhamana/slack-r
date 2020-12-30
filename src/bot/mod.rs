use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, Weekday};
use log::{debug, error, info, warn};
use rand::seq::SliceRandom;
use surf::{
    middleware::{Logger, Middleware, Next},
    Client, Request, Response, Url,
};

use crate::{
    API_KEY_ENV_NAME, CONFIG_FILE_PATH_ENV_VAR, DEFAULT_CONFIG_PATH, SLACK_API_URL, SlackRError, 
    api::{self, SlackApiContent, SlackApiError, SlackApiWarning},
    convert_date_string_to_local
};

mod config;
use config::BotConfig;
mod utils;
use utils::yes;

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
        let client = utils::create_client(token);
        debug!("Bot setup complete");
        SlackBot { client, config }
    }
    
    pub fn save(self) {
        match self.config.to_file() {
            Ok(_) => info!("Successfully saved file."),
            Err(_) => warn!("Couldnt' save file")
        }
    }

    pub async fn joke(self, input_date_arg: Option<&str>, scheduled_day_arg: Option<&str>) {
        info!("Processing joke command");        
        let target_date = self.get_target_date(input_date_arg);
        info!("Target datetime: {}.", target_date);
        
        let post_at = self.get_post_at_date(&target_date, scheduled_day_arg);
        info!("Message schedule datetime: {}. Timestamp {}", post_at, post_at.timestamp());
        
        if post_at <= Local::now() {
            error!("Too late to post for {}!", post_at);
            return;
        }
        
        debug!("Checking it isn't already scheduled for channel...");
        if self.date_already_been_scheduled(post_at).await {
            error!("This date has already been scheduled. Check with `scheduled` command, and/or cancel with the `delete <ID>` command.");
            return;
        };
        debug!("Nothing scheduled on {}, continuing", post_at);
        
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
        let response = self.schedule_message(post_at.timestamp(), text).await;
        println!("Successfully assigned member {} for a joke on {}. Message will be posted at {}. Schedule ID: {}", 
            member,
            target_date,
            response.post_at,
            response.scheduled_message_id
        );
    }

    async fn schedule_message(&self, timestamp: i64, message: String) -> api::ScheduleMessageResponse {
        let request = api::ScheduleMessageRequest::new(
            &self.config.channel, 
            timestamp,
            message);
        api::schedule_message(&self.client, &request).await
    }

    fn date_with_set_time(&self) -> DateTime<Local> {
        Local::today().and_time(self.config.target_time).expect("Couldn't generate time")
    }

    fn get_post_at_date(&self, target_date: &DateTime<Local>, post_at_arg: Option<&str>) -> DateTime<Local> {
        if let Some(post_at) = post_at_arg {
            debug!("A target time was specified");
            let time = self.date_with_set_time();
            let schedule_time = convert_date_string_to_local(&post_at, &time).unwrap();
            match schedule_time.cmp(target_date) {
                std::cmp::Ordering::Equal |
                std::cmp::Ordering::Greater => {
                    panic!("Scheduled time is after the target day!");
                },
                std::cmp::Ordering::Less => {
                    info!("valid target time specified: {}", schedule_time);
                    return schedule_time
                }
            }
        };
        debug!("Getting schedule time from target");
        let unfiltered = *target_date - Duration::seconds(self.config.target_time_schedule_offset);
        match unfiltered.date().weekday() {
            Weekday::Sun => {
                warn!("Offset falling on a sunday, shifting schedule to the Friday before");
                unfiltered - Duration::days(2)
            },
            _ => unfiltered
        }

    }

    fn get_target_date(&self, input_date_arg: Option<&str>) -> DateTime<Local> {
        let today_with_target_time = self.date_with_set_time();

        let unfiltered = match input_date_arg {
            Some(input_date_str) => {
                debug!("Date {} was input", input_date_str);
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
            Weekday::Sat => {
                warn!("Target date is a Saturday, shifting target to next Monday.");
                unfiltered + Duration::days(2)
            },
            Weekday::Sun => {
                warn!("Target date is a Sunday, shifting target to next Monday.");
                unfiltered + Duration::days(1)
            },
            _ => unfiltered
        }
    }

    pub async fn reroll(self) {
        let mut exclude = Vec::new();
        let mut selected_member;
        let len_limit = self.config.members.len();
        loop {
            selected_member = match self.select_random_member() {
                Some(m) => {
                    info!("Selected member {}", m);
                    m
                },
                None => {
                    error!("No member could be selected!");
                    return;
                }
            };
            if exclude.contains(&selected_member) {
                info!("Member already excluded. Automatically rerolling");
                continue
            };
            println!("Member {} was selected. Pick it?", selected_member);
            if yes() {
                break
            } else {
                exclude.push(selected_member);
                if exclude.len() + 1 == len_limit {
                    error!("You have excluded all members!");
                    return;
                };
            };
        };
        let target_date = self.get_target_date(None);
        let post_at = Local::now() + Duration::seconds(self.config.instant_delay);
        let message = format!(
            "Reroll: <@{}> will be in charge of a joke on {}!",
            selected_member,
            target_date.naive_local().date()
        );
        let response = self.schedule_message(post_at.timestamp(), message).await;
        println!("Successfully assigned member {} for a joke on {}. Message will be posted at {}. Schedule ID: {}", 
            selected_member,
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
        if let Some(token) = token_opt { 
            info!("Token: {}", token); 
            self.add_token(token).await;
        };
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


        if let Some(target_time) = target_time_opt {
            info!("Target time: {}", target_time);
            self.add_target_time(target_time);
        };
        println!("{:?}", self.config);
        let path = BotConfig::get_config_path();
        println!("Save to file at {:?}? y/n", path);
        if yes() {
            self.save();
        }
    }

    /// Takes a user email as input, fetches its ID and adds its ID to the config members (with confirmation for matching user)
    pub async fn add_member_from_email(&mut self, email: &str) {
        info!("Processing add member command");
        let request = api::UserLookupRequest{ email: email.to_string() };
        let response = api::call_endpoint(api::UserLookupByEmailEndpoint, &request, &self.client).await;
        match response.content {
            SlackApiContent::Ok(response) => {
                let name = response.user.profile.display_name.or(Some(response.user.name)).unwrap();
                println!("Found user {}. Is it who you want, save its ID {} in config? y/n", name, response.user.id);
                if yes() {
                    self.config.members.push(response.user.id);
                }
            },
            SlackApiContent::Err(slack_err) => {
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
        match join_channel_response.content {
            SlackApiContent::Ok(response) => {
                match join_channel_response.warning {
                    Some(SlackApiWarning::already_in_channel) => warn!("Was already in channel {}.", response.channel.name),
                    _=> info!("Successfully joined channel {}", response.channel.name)
                }
            },
            SlackApiContent::Err(err) => {
                error!("Couldn't join channel. Error: {:?}. Aborting.", err.error);
                return;
            }
        };

        let members = match api::list_members_for_channel(&self.client, &channel).await {
            Ok(m) => m,
            Err(err) => {
                match err {
                    SlackApiError::invalid_channel 
                    | SlackApiError::channel_not_found => error!("The channel {} is invalid. A channel ID can be acquired from the URL of a message quote.", channel),
                    _ => error!("Slack error: {:?}.", err)
                }
                warn!("Adding empty members list");
                vec![]
            }
        };
        self.config.members.extend(members
            .into_iter()
            .filter(|e| e != &self.config.id)
            .filter(|e| !self.config.members.contains(e))
            .collect::<Vec<String>>());
        // self.config.members = self.config.members.into_iter()
        //                                         .chain(members)
        //                                         .collect();
                                                
        self.config.channel = channel;
    }

    pub async fn add_token(&mut self, token: &str) {
        let new_client = utils::create_client(token.to_string());
        let request = api::Empty{};
        let identity = api::call_endpoint(api::AuthTestEndpoint, &request, &new_client).await;
        match identity.content {
            SlackApiContent::Ok(res) => {
                self.config.id = res.user_id;
                self.config.token = Some(token.to_string());
                self.client = new_client;
            },
            SlackApiContent::Err(err) => error!("Slack error: {:?}.", err)
        }
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

    async fn date_already_been_scheduled(&self, date: DateTime<Local>) -> bool {
        let messages = api::list_scheduled_messages(&self.client, &self.config.channel).await;
        messages.iter().any(|mess| mess.date() == date.date())
    }
    
    pub async fn cancel_scheduled_message(self, id: &str) {
        let messages = api::list_scheduled_messages(&self.client, &self.config.channel).await;
        debug!("Filtering from {} messages", messages.len());
        let lookup = messages.iter().find(|mess| mess.id == id);
        let message = match lookup {
            Some(mess) => {
                println!("Found message: {}", mess);
                println!("Please confirm cancellation: Y/n");
                if yes() {
                    mess
                } else {
                    warn!("Scheduled message kept.");
                    return;
                }
            }
            None => {
                error!("No scheduled message with id \"{}\"", id);
                return;
            }
        };
        let request = api::DeleteScheduledMessageRequest::new(&self.config.channel, &message.id);
        let response = api::call_endpoint(api::DeleteScheduledMessageEndpoint, &request, &self.client).await;
        match response.content {
            SlackApiContent::Ok(_empty) => println!("Deleted message with id {}", id),
            SlackApiContent::Err(err) => error!("Failed to delete: {:?}", err)
        }
    }
}
