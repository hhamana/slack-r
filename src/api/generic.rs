use super::*;

// ** Generic Utils **
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
    type Response: Serialize + DeserializeOwned + std::fmt::Debug;

    fn endpoint_url(&self) -> &str;
    fn method(&self) -> HttpVerb;
    fn build_request(&self, client: &Client, request: &Self::Request) -> surf::RequestBuilder {
        let data = serde_json::to_value(request).unwrap();
        debug!("JSON request {}", data);
        match self.method() {
            HttpVerb::POST => client.post(self.endpoint_url()).body(data),
            HttpVerb::GET => client
                .get(self.endpoint_url())
                .query(&data)
                .unwrap()
                .header(
                    surf::http::headers::CONTENT_TYPE,
                    format!("{}; charset=utf-8", surf::http::mime::FORM),
                ),
        }
    }
}

pub async fn call_endpoint<E: SlackEndpoint>(
    endpoint: E,
    request: &E::Request,
    client: &surf::Client,
) -> SlackApiResponse<E::Response> {
    info!("Calling {:?}", endpoint);
    let request = endpoint.build_request(client, request);
    let raw = request.recv_string().await.unwrap();
    debug!("Raw response: {}", raw);
    let response: SlackApiResponse<E::Response> = serde_json::from_str(&raw).unwrap();
    debug!("Serialized response: {:?}", response);
    match &response.content {
        SlackApiContent::Ok(_ok) => info!("Got Slack response successfully"),
        SlackApiContent::Err(e) => error!(
            "Slack responded with an error on the request for {}: {:?}",
            endpoint.endpoint_url(),
            e.error
        ),
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
    /// Not found
    method_not_found,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
#[non_exhaustive]
pub enum SlackApiWarning {
    missing_charset,
    already_in_channel,
    message_truncated,
    superfluous_charset,
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
    pub response_metadata: Option<ResponseMetadata>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SlackApiContent<T> {
    Ok(T),
    Err(SlackApiErrorResponse),
}

impl<T> SlackApiResponse<T> {
    /// Mimick a the Rust's Result type
    pub(crate) fn unwrap(self) -> T {
        match self.content {
            SlackApiContent::Ok(value) => value,
            SlackApiContent::Err(error) => panic!("{:?}", error),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub next_cursor: Option<String>,
    pub warnings: Option<Vec<SlackApiWarning>>,
}
