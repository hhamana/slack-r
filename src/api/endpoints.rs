use super::*;

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
    pub post_at: DateTime<Local>,
    pub message: MessageResponse,
}

impl From<ScheduleMessageResponseRaw> for ScheduleMessageResponse {
    fn from(mess: ScheduleMessageResponseRaw) -> Self {
        ScheduleMessageResponse {
            channel: mess.channel,
            scheduled_message_id: mess.scheduled_message_id,
            post_at: Local.timestamp(mess.post_at, 0),
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
    is_stranger: Option<bool>,
    updated: u64,
    is_app_user: bool,
    is_invited_user: Option<bool>,
    has_2fa: Option<bool>,
    locale: Option<String>
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
    fields: Option<Vec<String>>,
    status_text: Option<String>,
    status_emoji: Option<String>,
    status_expiration: Option<u64>,
    avatar_hash: Option<String>,
    // pub email: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    image_original: Option<String>,
    image_24: String,
    image_32: String,
    image_48: String,
    image_72: String,
    image_192: String,
    image_512: String,
    status_text_canonical: Option<String>,
    team: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Team {
    id: String,
    name: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLookupResponse {
    pub user: UserObject,
    pub team: Option<Team>,
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
    pub id: String,
    channel_id: String,
    post_at: DateTime<Local>,
    date_created: DateTime<Local>,
    text: String,
}

impl std::fmt::Display for ScheduledMessageObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ID: {}, created {}, scheduled for {} - #{}:  {}",
            self.id, self.date_created.to_rfc3339(), self.post_at.to_rfc3339(), self.channel_id,  self.text
            // "{}: {} - {} (Created at {})",
            // self.post_at, self.channel_id, self.text, self.date_created
        )
    }
}

impl From<&ScheduledMessageObjectRaw> for ScheduledMessageObject {
    fn from(raw: &ScheduledMessageObjectRaw) -> Self {
        ScheduledMessageObject {
            id: raw.id.clone(),
            channel_id: raw.channel_id.clone(),
            post_at: Local.timestamp(raw.post_at, 0),
            date_created: Local.timestamp(raw.date_created, 0),
            text: raw.text.clone(),
        }
    }
}
impl ScheduledMessageObject {
    pub fn date(&self) -> chrono::Date<Local> {
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


#[derive(Debug)]
pub struct DeleteScheduledMessageEndpoint;
impl SlackEndpoint for DeleteScheduledMessageEndpoint {
    type Request = DeleteScheduledMessageRequest;
    type Response = Empty;

    fn endpoint_url(&self) -> &str {
        "chat.deleteScheduledMessage"
    }

    fn method(&self) -> HttpVerb {
        HttpVerb::POST
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteScheduledMessageRequest {
    channel: String,
    scheduled_message_id: String,
}
impl DeleteScheduledMessageRequest {
    pub fn new(channel: &str, id: &str) -> DeleteScheduledMessageRequest {
        DeleteScheduledMessageRequest {
            channel: channel.to_string(),
            scheduled_message_id: id.to_string()
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Empty {}

#[derive(Debug)]
pub struct AuthTestEndpoint;

impl SlackEndpoint for AuthTestEndpoint {
    type Request = Empty;
    type Response = BotIdentity;
    fn endpoint_url(&self) -> &str {
        "auth.test"
    }

    fn method(&self) -> HttpVerb {
        HttpVerb::GET
    }

    fn build_request(&self, client: &Client, _request: &Self::Request) -> surf::RequestBuilder {
    // override so the empty request doesn't call a GET "/auth.test?"
        client.get(self.endpoint_url())
            .header(surf::http::headers::CONTENT_TYPE, format!("{}; charset=utf-8", surf::http::mime::FORM))
            
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BotIdentity {
    url: String,
    pub team: String,
    user: String,
    team_id: String,
    pub user_id: String,
    pub bot_id: String,
}
