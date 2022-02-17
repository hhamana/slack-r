mod endpoints;
mod generic;
use chrono::{DateTime, Local, TimeZone};
pub use endpoints::*;
pub use generic::*;
use log::{debug, error, info};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use surf::Client;
