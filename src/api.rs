use chrono::{DateTime, TimeZone, Utc, Local};
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
    type Response: Serialize + DeserializeOwned + std::fmt::Debug;

    fn endpoint_url(&self) -> &str;
    fn method(&self) -> HttpVerb;
}

pub async fn call_endpoint<E: SlackEndpoint>(
    endpoint: E,
    request: &E::Request,
    client: &surf::Client,
) -> SlackApiResponse<E::Response> {
// ) -> E::Response  {
    info!("Calling {:?}", endpoint);
    let data = serde_json::to_value(request).unwrap();
    debug!("JSON request {}", data);
    let request = match endpoint.method() {
        HttpVerb::POST => client.post(endpoint.endpoint_url()).body(data),
        HttpVerb::GET => client.get(endpoint.endpoint_url()).query(&data).unwrap()
            .header(surf::http::headers::CONTENT_TYPE, format!("{}; charset=utf-8", surf::http::mime::FORM)),
    };
    let raw = request.recv_string().await.unwrap();
    debug!("Raw response {}", raw);
    let response: SlackApiResponse<E::Response> = serde_json::from_str(&raw).unwrap();
    debug!("Serialized from JSON {:?}", response);
    match &response.content {
        SlackApiContent::Ok(_ok) =>  info!("Got Slack response successfully"),
        SlackApiContent::Err(e) =>  error!("Slack responded with an error on the request for {}: {:?}", endpoint.endpoint_url(), e.error),
    };
    
    response
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
#[non_exhaustive]
pub enum SlackApiError {
    ///Invalid user id provided
    invalid_user_id,
    /// The channel passed is invalid
    invalid_channel, 
    /// Value passed for channel was not found
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
    /// Method used wasn't recognized
    unknown_method,
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
#[allow(non_camel_case_types)]
#[non_exhaustive]
pub enum SlackApiWarning {
    missing_charset,
    already_in_channel,
    message_truncated,
    superfluous_charset
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlackApiErrorResponse {
    pub error: SlackApiError,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlackApiResponse<T> {
    /// Slack took great care of putting the ok to check for success, but I don't even need it, serde can tell from the fields.
    ok: bool,
    /// `content` is not a field that the API actually sends, but this allows to have a generic-ish struct. 
    // flattened response with fields as is, will be deserialized and put together under this field.
    #[serde(flatten)]
    pub content: SlackApiContent<T>,
    /// Sometimes some warnings. Logic won't do anythign about it, but it's good to know when developping.
    pub warning: Option<SlackApiWarning>,
    /// can have Metadata, or repeat warnings. Maybe other usages, but haven't seen yet, so aren't defined here, and won't be picked.
    /// report if any raw response sends more!
    pub response_metadata: Option<ResponseMetadata>

}
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SlackApiContent<T> {
    Ok(T),
    Err(SlackApiErrorResponse)
}

impl<T> SlackApiResponse<T> {
    pub(crate) fn unwrap(self) -> T {
        match self.content {
            SlackApiContent::Ok(value) => value,
            SlackApiContent::Err(error) => panic!("{:?}", error)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMetadata {
    next_cursor: Option<String>,
    warnings: Option<Vec<SlackApiWarning>>
}
//** Concrete implementations  **

// Schedule Message
#[derive(Debug)]
struct ScheduleMessageEndpoint;
impl SlackEndpoint for ScheduleMessageEndpoint {
    type Request = ScheduleMessageRequest;
    type Response = ScheduleMessageResponseRaw;

    fn endpoint_url(&self) -> &str { 
        "chat.scheduleMessage"
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
#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleMessageResponseRaw {
    channel: String,
    scheduled_message_id: String,
    post_at: i64,
    message: MessageResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub text: String,
    #[serde(alias = "username")]
    user: String,
    team: Option<String>,
    bot_id: Option<String>,
    #[serde(rename = "type")]
    type_: String,
    bot_profile: Option<BotProfile>,
    attachements: Option<Vec<Attachments>>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BotProfile {
    id: String, 
    deleted: bool,
    name: String, 
    updated: i64,
    app_id: String,
    icons: Icons,
    team_id: String
}

#[derive(Debug, Serialize, Deserialize)]
struct Icons {
    image_36: Option<String>,
    image_48: String,
    image_72: String
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Attachments {
    text: String,
    id: String,
    fallback: String,
}

#[derive(Serialize, Deserialize)]
pub struct ScheduleMessageResponse {
    pub channel: String,
    pub scheduled_message_id: String,
    pub post_at: DateTime<Utc>,
    pub message: MessageResponse,
}

impl From<ScheduleMessageResponseRaw> for ScheduleMessageResponse {
    fn from(mess: ScheduleMessageResponseRaw) -> Self {
        ScheduleMessageResponse {
            channel: mess.channel,
            scheduled_message_id: mess.scheduled_message_id,
            post_at: Utc.timestamp(mess.post_at, 0),
            message: mess.message,
        }
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

pub async fn list_members_for_channel(client: &Client, channel: &String) -> Result<Vec<String>, SlackApiError> {
    let mut members = Vec::new();
    let mut request = ListMembersRequestParams {channel: channel.clone(), cursor: None};
    loop {
        let full_response = call_endpoint(ListMembersEndpoint, &request, client).await;
        match full_response.content {
            SlackApiContent::Ok(response) => {
                members.extend(response.members);
                if let Some(metadata) = full_response.response_metadata {
                    if let Some(next_cursor) = metadata.next_cursor {
                        if !next_cursor.is_empty() {
                            request.cursor = Some(next_cursor);
                        } else {
                            break;
                        }
                    } else {
                        break;
                    };
                }
            },
            SlackApiContent::Err(err) => return Err(err.error)
        }
    };
    Ok(members)
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

pub async fn list_scheduled_messages(client: &Client, channel: &str) -> Vec<ScheduledMessageObject> {
    let mut request = ScheduledMessagesListRequest { channel: Some(channel.to_string()), ..ScheduledMessagesListRequest::default()};
    let mut all_responses = Vec::new();

    loop {
        let full_response = call_endpoint(ListScheduledMessagesEndpoint, &request, client).await;
        match full_response.content {
            SlackApiContent::Ok(response) => {
                let page_objects_iterator = response
                    .scheduled_messages
                    .iter()
                    .map(|element| ScheduledMessageObject::from(element));
                all_responses.extend(page_objects_iterator);
                debug!("Added to total, {} items", all_responses.len());
                if let Some(metadata) = full_response.response_metadata {
                    if let Some(next_cursor) = metadata.next_cursor {
                        if !next_cursor.is_empty() {
                            request.cursor = Some(next_cursor);
                        } else {
                            break;
                        }
                    } else {
                        break;
                    };
                }
            },
            SlackApiContent::Err(err) => {
                error!("{:?}", err);
                break;
            }
        }
    };

    debug!("Total {} scheduled message fetched", all_responses.len());
    all_responses
}

// List Of Pending Scheduled Messages
#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduledMessagesListRequest {
    channel: Option<String>,
    cursor: Option<String>,
    // latest: Option<i64>,
    // limit: Option<u64>,
    // oldest: Option<i64>,
}
impl Default for ScheduledMessagesListRequest {
    fn default() -> Self {
        ScheduledMessagesListRequest {
            channel: None,
            cursor: None,
            // latest: None,
            // limit: None,
            // oldest: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduledMessagesListRaw {
    scheduled_messages: Vec<ScheduledMessageObjectRaw>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduledMessageObjectRaw {
    channel_id: String,
    date_created: i64,
    id: String,
    post_at: i64,
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduledMessageObject {
    id: String,
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
            id: raw.id.clone(),
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ListMembersResponse {
    // List OF IDs
    pub members: Vec<String>,
}

//** Join Conversation Endpoint
#[derive(Debug)]
pub struct JoinConversationEndpoint;
impl SlackEndpoint for JoinConversationEndpoint {
    type Response = JoinConversationResponse;
    type Request = JoinConversationRequest;
    fn method(&self) -> HttpVerb {
        HttpVerb::POST
    }

    fn endpoint_url(&self) -> &str {
        "conversations.join"
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinConversationRequest {
    pub channel: String
}


#[derive(Debug, Serialize, Deserialize)]
pub struct JoinConversationResponse {
    pub channel: ChannelObject,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Warnings {
    warnings: Vec<String>
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelObject {
    pub id: String,
    pub name: String,
    is_channel: bool,
    is_group: bool,
    is_im: bool,
    created: i64,
    creator: String,
    is_archived: bool, 
    is_general: bool, 
    unlinked: i64,
    name_normalized: String, 
    is_shared: bool, 
    is_ext_shared: bool, 
    is_org_shared: bool, 
    pending_shared: Vec<String>,
    is_pending_ext_shared: bool, 
    is_member: bool, 
    is_private: bool, 
    is_mpim: bool, 
    topic: ChannelTopic,
    purpose: ChannelPurpose,
    previous_names: Vec<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelTopic {
    value: String,
    creator: String,
    last_set: i64
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelPurpose {
value: String,
creator: String,
last_set: i64,
}