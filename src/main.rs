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
        )
        .arg(Arg::with_name("list")
            .short("ls")
            .long("list")
            .takes_value(false)
            .help("Lists the scheduled messages")
    );

    let config_long_about = format!("Configuration is saved to a local file, defaulting to {} in the current folder, 
        the path and filename of which can be configured with the {} environnment variable.\n
        The values can be configured from the CLI (see OPTIONS), or directly manually from the file as json format.\n
        The command will read the data content, updated with the values and offer to save the file.", DEFAULT_CONFIG_PATH, CONFIG_FILE_PATH_ENV_VAR);
    let config_command = SubCommand::with_name("config")
        .about("Configures the bot")
        .long_about(config_long_about.as_str())
        .arg(Arg::with_name("members")
            .short("m")
            .long("members")
            .takes_value(true)
            .multiple(true)
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
        .subcommand(config_command);
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
    let config_command = matches.is_present("config");
    debug!("Config command? {}", config_command);
    let bot = SlackBot::new(config_command);
    info!("Bot initialized");

    debug!("Dispatching");
    match matches.subcommand() {
        ("joke", Some(args)) => {
            debug!("Joke subcommand");
            let input_date_arg = args.value_of("day");
            task::block_on(bot.joke(input_date_arg));
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
struct SlackRError;

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
