mod api;
mod bot;
mod dates;
use async_std::task;
use bot::{BotConfig, SlackBot};

use clap::{App, Arg, SubCommand};
use dates::{validate_date_input, validate_time_input};
use log::{debug, info, warn};
use simplelog::{Config, LevelFilter, SimpleLogger};

use crate::api::ProdSlackApiClient;

const API_KEY_ENV_NAME: &str = "SLACK_API_KEY";
const BOT_NAME: &str = "Slack-R";
const CLI_VERSION: &str = "0.1.4";
const CONFIG_FILE_PATH_ENV_VAR: &str = "SLACK_R_CONFIG_FILE_PATH";
const DEFAULT_CONFIG_PATH: &str = "./config.json";

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
            .help("Select a specific day to include in the message. Format as YYYY-MM-DD. Only dates in the future are allowed. Defaults to tomorrow. Can accept several dates in a single run"))
        .arg(Arg::with_name("post_on")
            .short("p")
            .long("post_on")
            .takes_value(true)
            .multiple(false)
            .validator(validate_date_input)
            .help("Select a specific day to schedule the message. 
Format as YYYY-MM-DD. Only dates in the future but before the --day argument allowed. 
Default to be calculated before the target day, before weekends. 
This arg allows overriding of the auto-calculated.
Currently unspecified behavior with several --day. Use only one when specificying the post date.")
        );
    let reroll_command = SubCommand::with_name("reroll")
        .about("Reroll for the next day")
        .help("Reroll for the next day, allowing you to preview the randomly selected name to filter out.");

    let add_member_command = SubCommand::with_name("member")
        .about("Adds a member ID to config, taking email as input to lookup Slack ID.")
        .arg(Arg::with_name("email")
            .required(true)
            .takes_value(true)
            .validator(validate_email)
            .help("Input the Slack's registered email of the user to add in config. The bot must have `users:read.email` permission")
        );

    // let add_token_command = SubCommand::with_name("token")
    //     .about("Adds a Slack API token to the config.")
    //     .arg(Arg::with_name("token")
    //         .required(true)
    //         .takes_value(true)
    //         .help("Saves the input token to config, so it doesn't need to eb set as environnment variable. A new token must be acquired form Slack, and will represent the bot's authentication and permissions")
    //     );

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
        .about("Sets the target time, post_at time, day offset. By default, both target time and post_at times are set to local 11:30 (AM).")
        .long_about("`target` time is the base from which the scheduled dateand time will be calculated. Set the `day_offset` to post one or more days before, at the time of `post_at`
        The joke command will abort operation if the current time is past the time to schedule.")
        .arg(Arg::with_name("target")
            .long("target")
            .takes_value(true)
            .help("Set the target time, which will be added to process the input date.")
        )
        .arg(Arg::with_name("post_at")
            .long("post_at")
            .takes_value(true)
            .help("Set the time at which to post at on the scheduled day.")
        )
        .arg(Arg::with_name("day_offset")
            .long("day_offset")
            .takes_value(true)
            .help("Sets how many days in advance to schedule relative to the target time.")
        );

    let add_command = SubCommand::with_name("add")
        .about("Adds various data to config, possibly fetching data from Slack")
        .subcommand(add_member_command)
        // .subcommand(add_token_command)
        .subcommand(add_channel_command)
        .subcommand(add_times_command);

    let scheduled_command =
        SubCommand::with_name("scheduled").about("Prints all scheduled messages for the bot.");

    let cancel_command = SubCommand::with_name("cancel")
        .about("Cancel scheduled messages from their IDs.")
        .long_about("Cancel scheduled messages from their IDs.\nThe ID is printed in succesful `joke` comand execution.\nAlternatively, you can get all scheduled messages IDs by using the `scheduled` command.\nWill ask for confirmation.")
        .arg(Arg::with_name("id")
            .takes_value(true)
            .required(true)
            .multiple(true)
            .help("Define the message ID to cancel.")
        );

    let config_long_about = format!("Configuration is saved to a local file, defaulting to {} in the current folder, 
        the path and filename of which can be configured with the {} environnment variable.\n
        The values can be configured from the CLI (see OPTIONS), or directly manually from the file as json format.\n
        The command will read the data content, updated with the values and offer to save the file.", DEFAULT_CONFIG_PATH, CONFIG_FILE_PATH_ENV_VAR);
    let config_command = SubCommand::with_name("config")
        .about("Configures the bot in bulk isntead of using separate adds")
        .long_about(config_long_about.as_str())
        .arg(
            Arg::with_name("members")
                .short("m")
                .long("members")
                .takes_value(true)
                .multiple(true)
                .validator(validate_email)
                .help("Adds a list of members"),
        )
        .arg(
            Arg::with_name("channel")
                .short("ch")
                .long("channel")
                .takes_value(true)
                .help("Register the channel to send scheduled messages to."),
        )
        .arg(
            Arg::with_name("target_time")
                .long("target_time")
                .takes_value(true)
                .validator(validate_time_input)
                .help("Register the time at which the message should be sheduled."),
        );
    // .arg(
    //     Arg::with_name("token")
    //         .long("token")
    //         .takes_value(true)
    //         .help("Registers the token in config file instead of env var."),
    // );
    let app = App::new(BOT_NAME)
        .version(CLI_VERSION)
        .about("Exposes command lines to control the Slack-R bot.")
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity, the more \"v\" the more verbose, up to -vvv."),
        )
        .subcommand(joke_command)
        .subcommand(reroll_command)
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

    let config = BotConfig::new();
    debug!("Created config");

    debug!("Looking for API token...");
    // Force crash if the api_key env var is not set right here. This is not an accident.
    let token: String = match (
        std::env::var(API_KEY_ENV_NAME),
        &config.token,
        should_panic) {
            // 3-way pattern matching to allow for many ways to get the token.
            // Isn't this beautiful?
            (Ok(var), _, _) => { debug!("Found token in {}", API_KEY_ENV_NAME); var},
            (_, Some(var), _ ) => { debug!("Found token in config"); var.clone()},
            (_, _,true) => { debug!("Didn't find token, but you get a free pass"); String::from("") },
            (_, _, _) => panic!("Token was not set. You can set it with the {} environnment variable, or using the `add token` command",
                    API_KEY_ENV_NAME)
    };
    let api = ProdSlackApiClient::new(token);
    let mut bot = SlackBot::new(config, api);
    info!("Bot initialized");

    debug!("Dispatching");
    match matches.subcommand() {
        ("joke", Some(args)) => {
            debug!("Joke subcommand");
            let input_date_args = args.values_of("day").unwrap_or_default().collect();
            let scheduled_day_arg = args.value_of("post_on");
            let scheduled = task::block_on(bot.joke(input_date_args, scheduled_day_arg));
            for joke in scheduled {
                println!("{}", joke);
            }
        }
        ("reroll", Some(_args)) => {
            debug!("Reroll subcommand");
            task::block_on(bot.reroll());
        }
        ("scheduled", _) => {
            debug!("Scheduled subcommand");
            task::block_on(bot.check_scheduled_messages());
        }
        ("cancel", Some(args)) => {
            debug!("Cancel subcommand");
            let id_values = args.values_of("id").unwrap().collect();
            task::block_on(bot.cancel_scheduled_message(id_values));
        }
        ("add", Some(args)) => {
            match args.subcommand() {
                ("member", Some(member_args)) => {
                    debug!("Add Member subcommand");
                    let email = member_args.value_of("email").unwrap();
                    task::block_on(bot.add_member_from_email(email));
                }
                ("channel", Some(channel_args)) => {
                    debug!("Add Channel subcommand");
                    let channel = channel_args.value_of("channel").unwrap();
                    task::block_on(bot.add_channel(channel));
                }
                ("time", Some(times_args)) => {
                    debug!("Add times subcommand");
                    let target_time_opt = times_args.value_of("target");
                    if let Some(target_time) = target_time_opt {
                        bot.add_target_time(target_time);
                    }
                    let post_at_opt = times_args.value_of("post_at");
                    if let Some(offset) = post_at_opt {
                        bot.add_post_time(offset);
                    }
                    let day_offset_opt = times_args.value_of("day_offset");
                    if let Some(offset) = day_offset_opt {
                        bot.set_post_day_offset(offset);
                    }
                }
                _ => panic!(
                    "Can only add channel, token or individual members! See `slack-r help add`"
                ),
            }
            bot.save();
        }
        ("config", Some(args)) => {
            debug!("Config subcommand");
            let members = args
                .values_of("members")
                .map(|members_val| members_val.map(|s| s.to_string()).collect());
            let channel = args.value_of("channel");
            let token = args.value_of("token");
            let target_time = args.value_of("target_time");
            task::block_on(bot.config(members, channel, token, target_time));
        }

        _ => panic!("No subcommand matching! See `slack-r help` for available commands."),
    };
    info!("Finished execution");
}

#[derive(Debug)]
pub enum SlackRError {
    TooLate,
    AlreadyScheduled,
    NoMemberToSelect,
    CorruptedConfig,
    WriteConfig,
}

fn validate_email(input_email: String) -> Result<(), String> {
    // Very naive email validation.
    let email_split: Vec<&str> = input_email.split('@').collect();
    if email_split.len() <= 1 {
        return Err("Not an email".to_string());
    }
    // let domain_split: Vec<&str> = email_split[1].split('.');
    if email_split[1].split('.').count() <= 1 {
        return Err("Not an email".to_string());
    }
    Ok(())
}
