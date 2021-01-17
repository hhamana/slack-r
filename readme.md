# Slack-r
This is a slack API Client built with the single purpose: assigning a joke to a member in a channel at a given time, scheduled to be posted with enough leeway time for the member to get it ready.
It does not store Slack message data. 
Configuration is saved to a file, which contain the selection of member IDs, the channel, and API token, as well as time preferences.

It has been designed to be usable by many separate people, pre-checking many steps to avoid duplications (and embarassment).

# Usage
This is a CLI to manage the bot. The bot is fundamentally nothing more than the api token.
Install, by downloading a release and putting it on your path, or invoking the executable portably.
See `slack-r help` for commands, and if you have the Rust tooling cargo, you can use `cargo doc --open` for some extra docs comment from the source code.

Here is the output of `slack-r help` as of 0.1.4, showing the high-level commands available. many subcommands have options with further help.
```
Slack-R 0.1.4
Exposes command lines to control the Slack-R bot.

USAGE:
    slack-r.exe [FLAGS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -v               Sets the level of verbosity, the more "v" the more verbose, up to -vvv.
    -V, --version    Prints version information

SUBCOMMANDS:
    add          Adds various data to config, possibly fetching data from Slack
    cancel       Cancel scheduled messages from their IDs.
    config       Configures the bot in bulk isntead of using separate adds
    help         Prints this message or the help of the given subcommand(s)
    joke         Notifies who has to find a joke.
    reroll       Reroll for the next day
    scheduled    Prints all scheduled messages for the bot.
```

### Verbosity
Verbosity has 3 levels, which technically are log levels.
When unset, it refers to ERROR level, so you will always see error messages.  
A single -v will trigger the WARN lavel, giving some information about some maybe unexpected behavior (such as weekend shifting).  
With -vv, you get INFO level, giving more information about which Slack endpoints it is calling, as well as how it processes times.
At -vvv, this is DEBUG level. Here, most functions will trigger some sort of log, so you can get a view of how everything goes through the system. Debug-only logs also show the module path of the code calling the log. It also triggers extensive logs from dependencies 

## Setup through commands
### add token <token>
First things first, add the Slack API token to configuration.
You only need to create a new token if a bot with the correct permissions hasn't been created for your organization.

### add channel
Adds a channel to the config.
It will:
- save the channel to config
- join the channel
- add the ID of all members in the channel to config
All usage is considered to be for this single target channel.
To use the bot in different channel, for now you can do so by creating different config files, and editing the ENV var accordingly.

## Environnment variables
```
SLACK_R_CONFIG_FILE_PATH
``` 
To specify the file path for the config path to use.  
Defaults to `./config.json`, so relative to invocation path.
Several various config files might be required to use the same bot in different channels.

```
SLACK_API_KEY
```
To specify the API key, aka token, for the bot.
Can also be defined in the config file. Env variable will take priority over config.



## Required Scopes

To use `add channel <channel>` a channel, the bot needs the `channels:join` scope permsssion. This allows to get all the channels's members ID necessarry to effectively mention/notify them when selected.
To use `add member <email>`, the bot neds to have `users:read.email` scope permission. This is optional, as long as you don't use it. The `add channel` will add add members in a batch so you probably don't need to cherry pick users.

# Technology
Proudly built in Rust, with async.