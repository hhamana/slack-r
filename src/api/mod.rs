use chrono::{DateTime, TimeZone, Local};
use log::{debug, error, info};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use surf::Client;

mod generic;
pub use generic::*;

mod endpoints;
pub use endpoints::*;