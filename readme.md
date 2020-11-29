# Slack-r
This is a slack API Client built with the single purpose: assigning a joke to a member in a channel at a given time, scheduled to be posted with enough leeway time for the member to get it ready.
It does not store Slack data.


# Usage
This is a CLI to manage the bot. The bot is fundamentally nothing more than the api token.
Install, by downloading a release and putting it on your Path, or invoking the executable portably. 
See `slack-r help` for commands, and if you have the Rust tooling cargo, you can use `cargo doc --open` for some extra docs comment from the source code.

## Environnment variables
```
SLACK_R_CONFIG_FILE_PATH
``` 
To specify the file path for the config path to use.  
Defaults to `./config.json`, so relative to invocation path.

```
SLACK_API_KEY
```
To specify the API key, aka token, for the bot.
Can also be defined in the config file. Env variable will take priority over config.



## Required Scopes

To use `add channel <channel>` a channel, the bot needs the `channels:join` scope permsssion. This allows to get all the channels's members ID necessarry to effectively mention/notify them when selected.
To use `add member <email>`, the bot neds to have `users:read.email` scope permission. This is optional, as long as you don't use it. The `add channel` will add add members in a batch so you probably don't need to cherry pick users.

# Technbology
Proudly built in Rust, with async.