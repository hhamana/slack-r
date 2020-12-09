use async_std::task;
use chrono::{DateTime, Local};
use clap::{App, Arg, SubCommand};
use log::{debug, info, warn};
use simplelog::{Config, LevelFilter, SimpleLogger};

mod api;
mod bot;
use bot::SlackBot;

const AUTHOR: &'static str = "Hamana Hadrien. <hamana.hadrien@gmail.com>";
const API_KEY_ENV_NAME: &'static str = "SLACK_API_KEY";
const BOT_NAME: &'static str = "Slack-R";
const CLI_VERSION: &'static str = "0.1";
const CONFIG_FILE_PATH_ENV_VAR: &'static str = "SLACK_R_CONFIG_FILE_PATH";
const DEFAULT_CONFIG_PATH: &'static str = "./config.json";
#[cfg(not(debug_assertions))]
const SLACK_API_URL: &'static str = "https://slack.com/api/";
#[cfg(debug_assertions)]
const SLACK_API_URL: &'static str = "http://localhost:3030";

/// Entry point and define command line interface.
fn main() {
    let joke_command = SubCommand::with_name("joke")
        .about("Notifies who has to find a joke.")
        .arg(Arg::with_name("day")
            .short("d")
            .long("day")
            .takes_value(true)
            .multiple(true)
            .validator(validate_date_input)
            .help("Select a specific day. Format as YYYY-MM-DD. Only dates in the future are allowed.")
        );
    
    let add_member_command = SubCommand::with_name("member")
        .about("Adds a member ID to config, taking email as input to lookup Slack ID.")
        .arg(Arg::with_name("email")
            .required(true)
            .takes_value(true)
            .validator(validate_email)
            .help("Input the Slack's registered email of the user to add in config. The bot must have `users:read.email` permission")
        );

    let add_token_command = SubCommand::with_name("token")
        .about("Adds a Slack API token to the config.")
        .arg(Arg::with_name("token")
            .required(true)
            .takes_value(true)
            .help("Saves the input token to config, so it doesn't need to eb set as environnment variable. A new token must be acquired form Slack, and will represent the bot's authentication and permissions")
        );

    let add_channel_command = SubCommand::with_name("channel")
        .about("Sets the channel to where the bot will post, and adds all the channels's users in config. Only one channel is allowed per configuration file, so any previously set channel will be overwitten. The bot will join the channel if not already in.")
        .arg(Arg::with_name("channel")
            .required(true)
            // .short("c")
            // .long("channel")
            .takes_value(true)
            .help("Specifies the channel to add")
        );

    let add_times_command = SubCommand::with_name("time")
        .about("Sets the target time and/or scheduling offset. By default, target time is set at 11:30 locally, with an offset of 23 hours.")
        .long_about(" This way, when the joke is invoked for the day 2020-12-01, it is calculated to be at 11:15, and schedule to post the message 23 hours before the target time. 
        The joke coimmand will abort operation if the current time is past the time to schedule.")
        .arg(Arg::with_name("target")
            .long("target")
            .takes_value(true)
            .help("Set the target time, which will be added to process the input date.")
        )
        .arg(Arg::with_name("offset")
            .long("offset")
            .takes_value(true)
            .help("Set the offset to apply for the target time.")
        );

    let add_command = SubCommand::with_name("add")
        .about("Adds various data to config, possibly fetching data from Slack")
        .subcommand(add_member_command)
        .subcommand(add_token_command)
        .subcommand(add_channel_command)
        .subcommand(add_times_command);
        
    let scheduled_command = SubCommand::with_name("scheduled")
        .about("Prints all scheduled messages for the bot.");
    
    let cancel_command = SubCommand::with_name("cancel")
        .about("Cancel a scheduled message from the ID.")
        .long_about("Cancel a scheduled message from the ID.\nThe ID is printed in succesful `joke` comand execution.\nAlternatively, you can get all scheduled messages IDs by using the `scheduled` command.\nWill ask for confirmation.")
        .arg(Arg::with_name("id")
            .takes_value(true)
            .required(true)
            .help("Define the message ID to cancel.")
        );

    let config_long_about = format!("Configuration is saved to a local file, defaulting to {} in the current folder, 
        the path and filename of which can be configured with the {} environnment variable.\n
        The values can be configured from the CLI (see OPTIONS), or directly manually from the file as json format.\n
        The command will read the data content, updated with the values and offer to save the file.", DEFAULT_CONFIG_PATH, CONFIG_FILE_PATH_ENV_VAR);
    let config_command = SubCommand::with_name("config")
        .about("Configures the bot in bulk isntead of using separate adds")
        .long_about(config_long_about.as_str())
        .arg(Arg::with_name("members")
            .short("m")
            .long("members")
            .takes_value(true)
            .multiple(true)
            .validator(validate_email)
            .help("Adds a list of members"),
        )
        .arg(Arg::with_name("channel")
            .short("ch")
            .long("channel")
            .takes_value(true)
            .help("Register the channel to send scheduled messages to.")
        )
        .arg(Arg::with_name("target_time")
            .long("target_time")
            .takes_value(true)
            .validator(validate_time_input)
            .help("Register the time at which the message should be sheduled.")
        )
        .arg(Arg::with_name("token")
            .long("token")
            .takes_value(true)
            .help("Registers the token in config file instead of env var.")
        );
    let app = App::new(BOT_NAME)
        .version(CLI_VERSION)
        .author(AUTHOR)
        .about("Exposes command lines to control the Slack-R bot.")
        .arg(Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity, the more \"v\" the more verbose, up to -vvv.")
        )
        .subcommand(joke_command)
        .subcommand(config_command)
        .subcommand(add_command)
        .subcommand(cancel_command)
        .subcommand(scheduled_command);
    // CLI defined,
    let matches = app.get_matches();

    let log_level = match matches.occurrences_of("v") {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };
    let _ = SimpleLogger::init(log_level, Config::default());

    warn!("Logging messages at {} level", log_level);

    //  will panick if you don't have the key as env var at this point.
    info!("Initializing bot");
    let should_panic = matches.is_present("config") || matches.is_present("add");
    debug!("No panic? {}", should_panic);
    let mut bot = SlackBot::new(should_panic);
    info!("Bot initialized");

    debug!("Dispatching");
    match matches.subcommand() {
        ("joke", Some(args)) => {
            debug!("Joke subcommand");
            let input_date_arg = args.value_of("day");
            task::block_on(bot.joke(input_date_arg));
        },
        ("scheduled", _) => {
            debug!("Scheduled subcommand");
            task::block_on(bot.check_scheduled_messages());
        },
        ("cancel", Some(args)) => {
            debug!("Cancel subcommand");
            let id = args.value_of("id").unwrap();
            task::block_on(bot.cancel_scheduled_message(id));
        },
        ("add", Some(args)) => {
            match args.subcommand() {
                ("member", Some(member_args)) => {
                    debug!("Add Member subcommand");
                    let email = member_args.value_of("email").unwrap();
                    task::block_on(bot.add_member_from_email(email));
                },
                ("token", Some(token_args)) => {
                    debug!("Add Token subcommand");
                    let token = token_args.value_of("token").unwrap();
                    task::block_on(bot.add_token(token));

                },
                ("channel", Some(channel_args)) => {
                    debug!("Add Channel subcommand");
                    let channel = channel_args.value_of("channel").unwrap();
                    task::block_on(bot.add_channel(channel));
                },
                ("time", Some(times_args)) => {
                    debug!("Add times subcommand");
                    let target_time_opt = times_args.value_of("target");
                    if let Some(target_time) = target_time_opt {
                        bot.add_target_time(target_time);
                    }
                    let offset_opt = times_args.value_of("target");
                    if let Some(offset) = offset_opt {
                        bot.add_offset_time(offset);
                    }
                }
                _ => panic!("Can only add channel, token or individual members! See `slack-r help add`"),
            }
            bot.save();
        },
        ("config", Some(args)) => {
            debug!("Config subcommand");
            let members = match args.values_of("members") {
                Some(members_val) => Some(members_val.map(|s| s.to_string()).collect()),
                None => None,
            };
            let channel = args.value_of("channel");
            let token = args.value_of("token");
            let target_time = args.value_of("target_time");
            task::block_on(bot.config(members, channel, token, target_time));
        },
        _ => panic!("No subcommand matching! See `slack-r help` for available commands."),
    };
    info!("Finished execution");
}

#[derive(Debug)]
pub struct SlackRError;

/// Takes a date string such as "2020-10-21" and returns a Datetime instance with local timezone and current time.
/// Returns a String as error, so it can be used to validate while invoking as command line argument too
fn convert_date_string_to_local(
    input_date: &str,
    today: &DateTime<Local>,
) -> Result<DateTime<Local>, String> {
    let input_plus_time = format!("{} {} {}", input_date, today.time(), today.offset());
    debug!("Processing time input as {}", input_plus_time);
    let parsed_date = input_plus_time.parse::<DateTime<Local>>()
        .map_err(|_e| format!("Not a date. Example format: {}", today.naive_local().date(),))?;
    debug!("Date successfully parsed");
    Ok(parsed_date)
}

fn validate_date_input(input_date: String) -> Result<(), String> {
    let today = Local::now();
    let parsed_date = convert_date_string_to_local(&input_date, &today)?;
    if parsed_date <= today {
        return Err(format!("Date {} must be in the future", input_date));
    }
    let in_120_days = today + chrono::Duration::days(120);
    if parsed_date >= in_120_days {
        return Err(
            format!("Date {} must not be more than 120 days in the future", input_date
        ));
    }
    debug!("Date confirmed valid");
    Ok(())
}

fn validate_time_input(input_time: String) -> Result<(), String> {
    match chrono::NaiveTime::parse_from_str(&input_time, "%H:%M:%S") {
        Ok(_v) => Ok(()),
        Err(e) => Err(format!("{}", e)),
    }
}


fn validate_email(input_email: String) -> Result<(), String> {
    // Very naive email validation. 
    let email_split: Vec<&str> = input_email.split("@").collect();
    if email_split.len() <= 1 { return Err("Not an email".to_string()) }
    let domain_split: Vec<&str> = email_split[1].split(".").collect();
    if domain_split.len() <= 1 { return Err("Not an email".to_string()) }
    Ok(())
}
