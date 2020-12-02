# Slack-r
This is a slack API Client built with the single purpose: assigning a joke to a member in a channel at a given time, scheduled to be posted with enough leeway time for the member to get it ready.
It does not store Slack message data. 
Configuration is saved to a file, which contain the selection of member IDs, the channel, and API token, as well as time preferences.

It has been designed to be usable by many separate people, pre-checking many steps to avoid duplications (and embarassment).

# Usage
This is a CLI to manage the bot. The bot is fundamentally nothing more than the api token.
Install, by downloading a release and putting it on your path, or invoking the executable portably.
See `slack-r help` for commands, and if you have the Rust tooling cargo, you can use `cargo doc --open` for some extra docs comment from the source code.

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