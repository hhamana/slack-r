use chrono::{DateTime, TimeZone, Utc};
use log::{debug, error, info};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use surf::Client;

// ** Generic Utils **
pub enum HttpVerb {
    GET,
    POST,
    // PUT,
    // DELETE
}

pub trait SlackEndpoint where Self: std::fmt::Debug {
    type Request: Serialize;
    type Response: Serialize + DeserializeOwned;

    fn endpoint_url(&self) -> &str;
    fn method(&self) -> HttpVerb;
}

pub async fn call_endpoint<E: SlackEndpoint>(
    endpoint: E,
    request: &E::Request,
    client: &surf::Client,
) -> SlackAPIResultResponse<E::Response>  {
// ) -> E::Response  {
    info!("Calling {:?}", endpoint);
    let data = serde_json::to_value(request).unwrap();
    debug!("JSON request {}", data);
    let request = match endpoint.method() {
        HttpVerb::POST => client.post(endpoint.endpoint_url()).body(data),
        HttpVerb::GET => client.get(endpoint.endpoint_url()).query(&data).unwrap(),
    };
    // let response: E::Response = request.recv_json().await.unwrap();
    let response: SlackAPIResultResponse<E::Response> = request.recv_json().await.unwrap();
    match &response {
        SlackAPIResultResponse::Ok(_ok) =>  info!("Got Slack response successfully"),
        SlackAPIResultResponse::Err(e) =>  error!("Slack responded with an error on the request for {}: {:?}", endpoint.endpoint_url(), e.error),
    };
    debug!("JSON response {}", serde_json::to_string(&response).unwrap());
    
    response
}

fn string_epoch_to_datetime(input_string: String) -> Result<DateTime<Utc>, ParseIntStringError> {
    let epoch = input_string
        .parse::<i64>()
        .map_err(|_| ParseIntStringError)?;
    let datetime = Utc.timestamp(epoch, 0);
    Ok(datetime)
}


#[derive(Debug, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum SlackApiError {
    ///Invalid user id provided
    invalid_user_id,

    /// The channel passed is invalid
    invalid_channel, 

    /// Value passed for channel was invalid.
    channel_not_found,
    
    /// Value passed for user was invalid.
    users_not_found,

    ///No authentication token provided.
    not_authed,

    /// Some aspect of authentication cannot be validated. Either the provided token is invalid or the request originates from an IP address disallowed from making the request.
    invalid_auth,

    /// This type of conversation cannot be used with this method.
    method_not_supported_for_channel_type,

    ///The token used is not granted the specific scope permissions required to complete this request.
    missing_scope,

    /// Authentication token is for a deleted user or workspace.
    account_inactive,
    ///Authentication token is for a deleted user or workspace or the app has been removed.
    token_revoked,

    /// The workspace token used in this request does not have the permissions necessary to complete the request. Make sure your app is a member of the conversation it's attempting to post a message to.
    no_permission,

    ///The workspace is undergoing an enterprise migration and will not be available until migration is complete.
    org_login_required,

    ///Administrators have suspended the ability to post a message.
    ekm_access_denied,

    /// The method cannot be called from an Enterprise.
    enterprise_is_restricted,

    /// Value passed for limit was invalid.
    invalid_limit,

    ///The token type used in this request is not allowed.
    not_allowed_token_type,

    /// Value passed for cursor was invalid.
    invalid_cursor, 

    /// Failed to fetch members for the conversation.
    fetch_members_failed,

    /// The method has been deprecated.
    method_deprecated,

    /// The endpoint has been deprecated.
    deprecated_endpoint,

    /// Two factor setup is required.
    two_factor_setup_required,

    /// This method cannot be called by a bot user.
    is_bot,

    /// The method was either called with invalid arguments or some detail about the arguments passed are invalid, which is more likely when using complex arguments like blocks or attachments.
    invalid_arguments,

    /// The method was passed an argument whose name falls outside the bounds of accepted or expected values. This includes very long names and names with non-alphanumeric characters other than _. If you get this error, it is typically an indication that you have made a very malformed API call.
    invalid_arg_name,

    /// The method was passed an array as an argument. Please only input valid strings.
    invalid_array_arg,
    
    /// The method was called via a POST request, but the charset specified in the Content-Type header was invalid. Valid charset names are: utf-8 iso-8859-1.
    invalid_charset,

    /// The method was called via a POST request with Content-Type application/x-www-form-urlencoded or multipart/form-data, but the form data was either missing or syntactically invalid.
    invalid_form_data,

    /// The method was called via a POST request, but the specified Content-Type was invalid. Valid types are: application/json application/x-www-form-urlencoded multipart/form-data text/plain.
    invalid_post_type,

    ///The method was called via a POST request and included a data payload, but the request did not include a Content-Type header.
    missing_post_type,

    /// The workspace associated with your request is currently undergoing migration to an Enterprise Organization. Web API and other platform operations will be intermittently unavailable until the transition is complete.
    team_added_to_org,

    /// The request has been ratelimited. Refer to the Retry-After header for when to retry the request.
    ratelimited,

    /// Access to this method is limited on the current network
    accesslimited,

    /// The method was called via a POST request, but the POST data was either missing or truncated.
    request_timeout,

    /// The service is temporarily unavailable
    service_unavailable,

    /// The server could not complete your operation(s) without encountering a catastrophic error. It's possible some aspect of the operation succeeded before the error was raised.
    fatal_error,

    /// Complete Slack API failure
    internal_error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlackApiErrorResponse {
    ok: bool,
    pub error: SlackApiError
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SlackAPIResultResponse<T> {
    Ok(T),
    Err(SlackApiErrorResponse)
}

impl<T> SlackAPIResultResponse<T> {
    pub(crate) fn unwrap(self) -> T {
        match self {
            SlackAPIResultResponse::Ok(value) => value,
            SlackAPIResultResponse::Err(error) => panic!("{:?}", error)
        }
    }
}

//** Concrete implementations  **

// Schedule Message
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

pub async fn schedule_message(
    client: &Client,
    schedule_message: &ScheduleMessageRequest,
) -> ScheduleMessageResponse {
    let response_res = call_endpoint(ScheduleMessageEndpoint, schedule_message, client).await;
    // ScheduleMessageResponse::from(response_res)
    ScheduleMessageResponse::from(response_res.unwrap())
}

// Schedule Message Structs
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
// </> Schedule message

// User Lookup By Email

#[derive(Debug)]
pub struct UserLookupByEmailEndpoint;
impl SlackEndpoint for UserLookupByEmailEndpoint {
    type Request = UserLookupRequest;
    type Response = UserLookupResponse;
    fn endpoint_url(&self) -> &str {
        "users.lookupByEmail"
    }
    fn method(&self) -> HttpVerb {
        HttpVerb::GET
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLookupRequest {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserObject {
    pub id: String,
    team_id: String,
    pub name: String,
    deleted: bool,
    color: String,
    real_name: String,
    tz: String,
    tz_label: String,
    tz_offset: i64,
    pub profile: UserProfile,
    is_admin: bool,
    is_owner: bool,
    is_primary_owner: bool,
    is_restricted: bool,
    is_ultra_restricted: bool,
    is_bot: bool,
    is_stranger: bool,
    updated: u64,
    is_app_user: bool,
    is_invited_user: bool,
    has_2fa: bool,
    locale: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    title: Option<String>,
    phone: Option<String>,
    skype: Option<String>,
    real_name: Option<String>,
    real_name_normalized: Option<String>,
    pub display_name: Option<String>,
    display_name_normalized: Option<String>,
    status_text: Option<String>,
    status_emoji: Option<String>,
    status_expiration: Option<u64>,
    avatar_hash: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    pub email: Option<String>,
    image_original: Option<String>,
    image_24: String,
    image_32: String,
    image_48: String,
    image_72: String,
    image_192: String,
    image_512: String,
    team: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Team {
    id: String,
    name: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLookupResponse {
    ok: bool,
    pub user: UserObject,
    // pub team: Team,
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
        let response = call_endpoint(ListScheduledMessagesEndpoint, &request, client).await.unwrap();
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
