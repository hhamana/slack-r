use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, Weekday};
use log::{debug, error, info, warn};
use rand::seq::SliceRandom;
use surf::Client;

use crate::{
    API_KEY_ENV_NAME,
    // SlackRError,
    api::{self, SlackApiContent, SlackApiError, SlackApiWarning},
    convert_date_string_to_local
};

mod config;
use config::BotConfig;
mod client;

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
                // 3-way pattern matching to allow for many ways to get the token.
                // Isn't this beautiful?
                (Ok(var), _, _) => { debug!("Found token in {}", API_KEY_ENV_NAME); var},
                (_, Some(var), _ ) => { debug!("Found token in config"); var.clone()},
                (_, _,true) => { debug!("Didn't find token, but you get a free pass"); String::from("") },
                (_, _, _) => panic!("Token was not set. You can set it with the {} environnment variable, or using the `add token` command",
                        API_KEY_ENV_NAME)
        };

        debug!("Creating Internet client");
        let client = client::create_client(token);
        debug!("Bot setup complete");
        SlackBot { client, config }
    }
    
    pub fn save(self) {
        match self.config.to_file() {
            Ok(_) => info!("Successfully saved config file."),
            Err(_) => error!("Couldnt' save config file")
        }
    }

    pub async fn joke(
        self, 
        input_date_args: Vec<&str>,
        scheduled_day_arg: Option<&str>
    ) {
        info!("Processing joke command");

        let target_datetimes: Vec<DateTime<Local>> = self.get_target_dates(input_date_args);
        debug!("Target dates: {:?}", target_datetimes);


        let already_scheduled_messages = api::list_scheduled_messages(&self.client, &self.config.channel).await;
        let mut messages_to_schedule: Vec<api::ScheduleMessageRequest> = Vec::new();

        for target_date in target_datetimes {
            info!("Target datetime: {}.", target_date);
            
            let post_at = self.get_post_at_date(&target_date, scheduled_day_arg);
            info!("Message schedule datetime: {}. Timestamp {}", post_at, post_at.timestamp());
            
            if post_at <= Local::now() {
                error!("Too late to post for {}!", post_at);
                continue
                // return Err(SlackRError::TooLate);
            }
            
            debug!("Checking it isn't already scheduled for channel...");
            if already_scheduled_messages.iter().any(|mess| mess.date() == post_at.date()) {
                error!("{} has already been scheduled. Check with `scheduled` command, and/or cancel with the `delete <ID>` command.",
                    post_at.date()
                );
                continue
                // return Err(SlackRError::AlreadyScheduled);
            };
            debug!("Confirmed nothing already scheduled on {}", post_at);
            
            debug!("Checking it isn't already scheduled in this batch");
            // It is okay to compare timestamps as they both also get the same time assigned.
            if messages_to_schedule.iter().any(|req| req.post_at == post_at.timestamp()) {
                error!("Attempting to schedule {} several times.",
                    post_at.date()
                );
                continue
                // return Err(SlackRError::AlreadyScheduled);
            };
            debug!("Confimed bot duplicating requests");
            
            let member = match self.select_random_member() {
                Some(m) => {
                    info!("Selected member {}", m);
                    m
                },
                None => {
                    error!("No member could be selected!");
                    continue;
                    // return Err(SlackRError::NoMemberToSelect);
                }
            };
            
            let text = format!(
                "<@{}> will be in charge of a joke on {}!",
                member,
                target_date.naive_local().date()
            );

            let request = api::ScheduleMessageRequest::new(
                &self.config.channel, 
                post_at.timestamp(),
                text
            );
            messages_to_schedule.push(request);
        };
        debug!("Messages to post: {:?}", messages_to_schedule);
        for message in messages_to_schedule {
            let response = api::schedule_message(&self.client, &message).await;
            println!("Message \"{}\" successully scheduled at {}. Schedule ID: {}", 
                message.text,
                response.post_at,
                response.scheduled_message_id
            );
        };
    }

    async fn schedule_message(&self, timestamp: i64, message: String) -> api::ScheduleMessageResponse {
        let request = api::ScheduleMessageRequest::new(
            &self.config.channel, 
            timestamp,
            message);
        api::schedule_message(&self.client, &request).await
    }

    fn get_post_at_date(&self, target_date: &DateTime<Local>, post_on_day_arg: Option<&str>) -> DateTime<Local> {
        if let Some(post_on_day) = post_on_day_arg {
            debug!("`post_on_day` was specified");
            let today_with_post_time = today_with_set_time(self.config.post_time);
            let post_at_time = convert_date_string_to_local(&post_on_day, &today_with_post_time).unwrap();
            match post_at_time.cmp(target_date) {
                std::cmp::Ordering::Equal |
                std::cmp::Ordering::Greater => {
                    panic!("Scheduled time is after the target day!");
                },
                std::cmp::Ordering::Less => {
                    info!("Valid post_at datetime specified: {}", post_at_time);
                    return post_at_time
                }
            }
        };
        debug!("Getting schedule time from target");
        let unfiltered = *target_date - Duration::days(self.config.advance_days);
        match unfiltered.date().weekday() {
            Weekday::Sun => {
                warn!("Offset falling on a sunday, shifting schedule to the Friday before");
                unfiltered - Duration::days(2)
            },
            // As advance_day can now be anything, it might fall on a saturday if target time falls on monday and offset 2 days.
            Weekday::Sat => {
                warn!("Offset falling on a saturday, shifting schedule to the Friday before");
                unfiltered - Duration::days(1)
            },
            _ => unfiltered
        }

    }

    fn get_target_dates(&self, input_date_args: Vec<&str>) -> Vec<DateTime<Local>> {
        let today_with_target_time = today_with_set_time(self.config.target_time);
        let mut unfiltered_dates = Vec::new();
        if input_date_args.is_empty() {
            debug!("No date was input. Getting tomorrow.");
            let tomorrow = today_with_target_time.date().succ().and_time(self.config.target_time).unwrap();
            unfiltered_dates.push(tomorrow);
        } else {
            for input_date_str in input_date_args {
                let target = convert_date_string_to_local(input_date_str, &today_with_target_time).unwrap();
                unfiltered_dates.push(target);
            } 
        }
        debug!("Unfiltered target dates: {:?}", unfiltered_dates);

        let mut all_dates = Vec::new();
        for unfiltered in unfiltered_dates {
            match unfiltered.date().weekday() {
                // weekend dates shifted to next monday
                Weekday::Sat => {
                    warn!("Target date is a Saturday, shifting target to next Monday.");
                    all_dates.push(unfiltered + Duration::days(2));
                },
                Weekday::Sun => {
                    warn!("Target date is a Sunday, shifting target to next Monday.");
                    all_dates.push(unfiltered + Duration::days(1));
                },
                _ => all_dates.push(unfiltered)
            }
        };
        all_dates
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
        let empty_vec = Vec::new();
        let target_dates = self.get_target_dates(empty_vec);
        let target_date = target_dates.first().unwrap();
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
                    SlackApiError::users_not_found => error!("User email was not found, or the bot doesn't have access to it."),
                    SlackApiError::missing_scope => {
                        error!("Usage of lookup by email requires the Slack `users:read.email` scope. Please verify bot permissions.")
                    },
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
        let join_channel_response = api::call_endpoint(
            api::JoinConversationEndpoint, &request, &self.client
        ).await;
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
                    | SlackApiError::channel_not_found => {
                        error!("The channel {} is invalid. A channel ID can be acquired from the URL of a message quote.", channel)
                    },
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
        let new_client = client::create_client(token.to_string());
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

    pub fn add_post_time(&mut self, offset: &str) {
        self.config.post_time = NaiveTime::parse_from_str(offset, "%H:%M:%S")
            .expect("Unable to parse offset time");
    }

    pub fn set_post_day_offset(&mut self, offset: &str) {
        self.config.advance_days =  offset.parse::<i64>().expect("Day offset not parsable to i64");
    }

    pub async fn check_scheduled_messages(self) {
        let mut messages = api::list_scheduled_messages(&self.client, &self.config.channel).await;
        info!("Printing {} scheduled messages for channel {}", messages.len(), self.config.channel);
        messages.sort_by(|a, b| a.post_at.cmp(&b.post_at));
        for mess in messages {
            println!("{}", mess);
        }
    }

    pub async fn cancel_scheduled_message(self, id_list: Vec<&str>) {
        let messages = api::list_scheduled_messages(&self.client, &self.config.channel).await;
        debug!("Filtering from {} messages", messages.len());
        for id in id_list {
            let lookup = messages.iter().find(|mess| mess.id == id);
            let message = match lookup {
                Some(mess) => {
                    println!("Found message: {}", mess);
                    println!("Please confirm cancellation: Y/n");
                    if yes() {
                        mess
                    } else {
                        warn!("Scheduled message kept.");
                        continue;
                    }
                }
                None => {
                    error!("No scheduled message with id \"{}\"", id);
                    continue;
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
}



fn today_with_set_time(time: NaiveTime) -> DateTime<Local> {
    Local::today().and_time(time).expect("Couldn't generate time")
}


pub(crate) fn yes() -> bool {
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

trait IsWeekday {
    fn is_weekday(&self) -> bool;
}
impl IsWeekday for DateTime<Local> {
    fn is_weekday(&self) -> bool {   
        let weekdays = vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri];
        let target_weekday = self.date().weekday();
        weekdays.contains(&target_weekday)
    }
}
impl IsWeekday for chrono::NaiveDate {
    fn is_weekday(&self) -> bool {   
        let weekdays = vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri];
        let target_weekday = self.weekday();
        weekdays.contains(&target_weekday)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::prelude::*;
    // use async_std::task;
    use client::create_client;
    
    fn custom_bot(target_time_str: &str, post_time_str: &str) -> SlackBot {
        let target_time = target_time_str.parse::<NaiveTime>().unwrap();
        let post_time = post_time_str.parse::<NaiveTime>().unwrap();

        let config = BotConfig {
            members: vec!["user_1".to_string(), "user_2".to_string(), "user3".to_string()],
            selected: vec![],
            channel: "test_channel".to_string(),
            target_time,
            post_time,
            advance_days: 1,
            instant_delay: 45,
            token: Some("test_token".to_string()),
            id: "test_bot_id".to_string()
        };
        let client = create_client("test_token".to_string());
        SlackBot { client, config }
    }

    #[test]
    fn it_works() {
        assert!(1+1 == 2)
    }
    
    #[test]
    fn get_target_date_default() {
        let bot = custom_bot("11:30:00", "11:30:00");
        assert_eq!(bot.config.target_time, NaiveTime::from_hms(11, 30, 00));
        let target_date = bot.get_target_dates(vec!["2021-12-31"]);
        let expected = Local.ymd(2021, 12, 31).and_hms(11, 30, 00);
        assert_eq!(target_date.first().unwrap().to_owned(), expected)
    }

    // #[test]
    // fn test_joke_success() {
    //     let post_time_config = NaiveTime::from_hms(1, 2, 3);
    //     let bot = custom_bot("02:03:04", &post_time_config.to_string());
    //     let mut next_weekday = Local::now().date().naive_local().succ();
    //     while !next_weekday.is_weekday() {
    //         next_weekday = next_weekday.succ();
    //     };

    //     let tomorrow = Local::now().date().naive_local().succ();
    //     let input_date_arg = vec![&tomorrow.to_string()];
    //     // assert_eq!(input_date_arg, "2021-01-21");
    //     let joke = task::block_on(bot.joke(input_date_arg, None));
    //     println!("{:?}", joke);
    //     assert!(joke.is_ok());
    //     let (member, target_date, post_at) = joke.unwrap();
    //     assert!(target_date.is_weekday());
    //     assert!(post_at.is_weekday());
    //     assert_eq!(post_at.hour(), post_time_config.hour());
    //     assert_eq!(post_at.minute(), post_time_config.minute());
    // }
}