mod client;
mod endpoints;
mod generic;
use chrono::{DateTime, Local, TimeZone};
pub(crate) use client::{ProdSlackApiClient, SlackApiClient};
pub use endpoints::*;
pub use generic::*;
use log::{debug, error, info};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use surf::Client;

#[cfg(not(debug_assertions))]
const SLACK_API_URL: &str = "https://slack.com/api/";
#[cfg(debug_assertions)]
const SLACK_API_URL: &str = "http://localhost:3030";

#[cfg(test)]
pub use client::tests::TestSlackClient;
