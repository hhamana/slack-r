use chrono::{DateTime, TimeZone, Utc};
use log::{debug, info};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use surf::Client;

pub enum HttpVerb {
    GET,
    POST,
    // PUT,
    // DELETE
}

pub trait SlackEndpoint
where
    Self: std::fmt::Debug,
{
    type Request: Serialize;
    type Response: Serialize + DeserializeOwned;

    fn endpoint_url(&self) -> &str;
    fn method(&self) -> HttpVerb;
}

pub async fn call_endpoint<E: SlackEndpoint>(
    endpoint: E,
    request: &E::Request,
    client: &surf::Client,
) -> E::Response {
    info!("Calling {:?}", endpoint);
    let data = serde_json::to_value(request).unwrap();
    debug!("JSON request {}", data);
    let request = match endpoint.method() {
        HttpVerb::POST => client.post(endpoint.endpoint_url()).body(data),
        HttpVerb::GET => client.get(endpoint.endpoint_url()).query(&data).unwrap(),
    };
    let response = request.recv_json::<E::Response>().await.unwrap();
    info!("Got Slack response successfully");
    debug!("JSON response {}", 
        serde_json::to_string(&response).expect("Serialization failure after receiving response")
    );
    response
}

#[derive(Debug)]
struct ScheduleMessageEndpoint;
impl SlackEndpoint for ScheduleMessageEndpoint {
    type Request = ScheduleMessageRequest;
    type Response = ScheduleMessageResponseRaw;

    fn endpoint_url(&self) -> &str { 
        "chat.ScheduleMessage" 
    }
    fn method(&self) -> HttpVerb { 
        HttpVerb::POST
    }
    
}

#[derive(Debug)]
struct UserIdentityEndpoint;
impl SlackEndpoint for UserIdentityEndpoint {
    type Request = UserIdentiyRequest;
    type Response = UserIdentiyResponse;
    fn endpoint_url(&self) -> &str {
        "users.identity"
    }
    fn method(&self) -> HttpVerb {
        HttpVerb::GET
    }
}

#[derive(Debug)]
pub struct ListMembersEndpoint;
impl SlackEndpoint for ListMembersEndpoint {
    type Request = ListMembersRequestParams;
    type Response = ListMembersResponse;

    fn endpoint_url(&self) -> &str {
        "conversations.members"
    }
    fn method(&self) -> HttpVerb {
        HttpVerb::GET
    }
}

pub async fn schedule_message(
    client: &Client,
    schedule_message: &ScheduleMessageRequest,
) -> ScheduleMessageResponse {
    let response = call_endpoint(ScheduleMessageEndpoint, schedule_message, client).await;
    ScheduleMessageResponse::from(response)
}

#[derive(Debug)]
struct ListScheduledMessagesEndpoint;
impl SlackEndpoint for ListScheduledMessagesEndpoint {
    type Request = ScheduledMessagesListRequest;
    type Response = ScheduledMessagesListRaw;
    fn method(&self) -> HttpVerb {
        HttpVerb::POST
    }
    fn endpoint_url(&self) -> &str {
        "chat.scheduledMessages.list"
    }
}

pub async fn list_scheduled_messages(client: &Client) -> Vec<ScheduledMessageObject> {
    let mut request = ScheduledMessagesListRequest::default();
    let mut all_responses = Vec::new();

    loop {
        let response = call_endpoint(ListScheduledMessagesEndpoint, &request, client).await;
        let page_objects_iterator = response
            .scheduled_messages
            .iter()
            .map(|element| ScheduledMessageObject::from(element));
        all_responses.extend(page_objects_iterator);
        debug!("Added to total, {} items", all_responses.len());
        if let Some(next_cursor) = response.response_metadata.next_cursor {
            if !next_cursor.is_empty() {
                request.cursor = Some(next_cursor)
            } else {
                break;
            }
        } else {
            break;
        };
    }

    debug!("Total {} scheduled message fetched", all_responses.len());
    all_responses
}

// ScheduleMessage
#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleMessageRequest {
    pub channel: String,
    pub post_at: i64,
    pub text: String,
}

impl ScheduleMessageRequest {
    pub fn new(channel: &String, post_at: i64, text: String) -> Self {
        ScheduleMessageRequest {
            channel: channel.clone(),
            post_at,
            text,
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct ScheduleMessageResponseRaw {
    ok: bool,
    channel: String,
    scheduled_message_id: String,
    post_at: String,
    message: MessageResponse,
}

#[derive(Serialize, Deserialize)]
pub struct MessageResponse {
    text: String,
    username: String,
    bot_id: String,
    attachments: Vec<Attachments>,
    #[serde(rename = "type")]
    type_: String,
    subtype: String,
}

#[derive(Serialize, Deserialize)]
pub struct Attachments {
    text: String,
    id: u64,
    fallback: String,
}

#[derive(Debug)]
struct ParseIntStringError;
impl std::fmt::Display for ParseIntStringError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Failed to parse the String as i64 Integer")
    }
}
impl std::error::Error for ParseIntStringError {}

fn string_epoch_to_datetime(input_string: String) -> Result<DateTime<Utc>, ParseIntStringError> {
    let epoch = input_string
        .parse::<i64>()
        .map_err(|_| ParseIntStringError)?;
    let datetime = Utc.timestamp(epoch, 0);
    Ok(datetime)
}

#[derive(Serialize, Deserialize)]
pub struct ScheduleMessageResponse {
    ok: bool,
    channel: String,
    scheduled_message_id: String,
    post_at: DateTime<Utc>,
    message: MessageResponse,
}

impl From<ScheduleMessageResponseRaw> for ScheduleMessageResponse {
    fn from(mess: ScheduleMessageResponseRaw) -> Self {
        ScheduleMessageResponse {
            ok: mess.ok,
            channel: mess.channel,
            scheduled_message_id: mess.scheduled_message_id,
            post_at: string_epoch_to_datetime(mess.post_at).unwrap(),
            message: mess.message,
        }
    }
}

impl std::fmt::Display for ScheduleMessageResponse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}",
            serde_json::to_string_pretty(&self).unwrap()
        )
    }
}

// List Of Pending Scheduled Messages
#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduledMessagesListRequest {
    channel: Option<String>,
    cursor: Option<String>,
    latest: Option<i64>,
    limit: Option<u64>,
    oldest: Option<i64>,
}
impl Default for ScheduledMessagesListRequest {
    fn default() -> Self {
        ScheduledMessagesListRequest {
            channel: None,
            cursor: None,
            latest: None,
            limit: None,
            oldest: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ScheduledMessagesListRaw {
    ok: bool,
    scheduled_messages: Vec<ScheduledMessageObjectRaw>,
    response_metadata: Pagination,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination {
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduledMessageObjectRaw {
    id: u64,
    channel_id: String,
    post_at: i64,
    date_created: i64,
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduledMessageObject {
    id: u64,
    channel_id: String,
    post_at: DateTime<Utc>,
    date_created: DateTime<Utc>,
    text: String,
}

impl std::fmt::Display for ScheduledMessageObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} - {} (Created at {})",
            self.post_at, self.channel_id, self.text, self.date_created
        )
    }
}

impl From<&ScheduledMessageObjectRaw> for ScheduledMessageObject {
    fn from(raw: &ScheduledMessageObjectRaw) -> Self {
        ScheduledMessageObject {
            id: raw.id,
            channel_id: raw.channel_id.clone(),
            post_at: Utc.timestamp(raw.post_at, 0),
            date_created: Utc.timestamp(raw.date_created, 0),
            text: raw.text.clone(),
        }
    }
}
impl ScheduledMessageObject {
    pub fn date(&self) -> chrono::Date<Utc> {
        self.post_at.date()
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ListMembersRequestParams {
    channel: String,
    cursor: Option<String>,
    // limit: Option<u64>
}

impl ListMembersRequestParams {
    pub fn new(channel: &str) -> ListMembersRequestParams {
        ListMembersRequestParams {
            channel: channel.to_string(),
            cursor: None,
            // limit: None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListMembersResponse {
    ok: bool,
    pub members: Vec<String>,
    response_metadata: Pagination,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserIdentiyRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    name: String,
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Team {
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserIdentiyResponse {
    ok: bool,
    user: User,
    team: Team,
}
