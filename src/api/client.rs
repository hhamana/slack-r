use super::*;
use async_trait::async_trait;
use std::convert::TryInto;
use surf::{
    middleware::{Logger, Middleware, Next},
    Config, Request, Response, Url,
};
struct HeadersMiddleware {
    token: String,
}

#[surf::utils::async_trait]
impl Middleware for HeadersMiddleware {
    async fn handle(
        &self,
        mut req: Request,
        client: Client,
        next: Next<'_>,
    ) -> Result<Response, surf::Error> {
        req.insert_header(
            surf::http::headers::AUTHORIZATION,
            format!("Bearer {}", self.token),
        );
        req.insert_header(
            surf::http::headers::CONTENT_TYPE,
            format!("{}; charset=utf-8", surf::http::mime::JSON),
        );
        let res = next.run(req, client).await?;
        Ok(res)
    }
}

pub(crate) fn create_client(token: String) -> Client {
    let headers = HeadersMiddleware { token };
    let client: Client = Config::new()
        .set_base_url(Url::parse(SLACK_API_URL).unwrap())
        .try_into()
        .unwrap();
    client.with(Logger::new()).with(headers)
}

/// Intermediate representation to interact with the Slack API's endpoint,
/// while still allowing for overrding for tests or local mock server
#[async_trait(?Send)]
pub trait SlackApiClient {
    async fn schedule_message(&self, request: &ScheduleMessageRequest) -> ScheduleMessageResponse;

    async fn join_conversation(
        &self,
        request: &JoinConversationRequest,
    ) -> SlackApiResponse<JoinConversationResponse>;
    async fn user_lookup_by_email(
        &self,
        request: &UserLookupRequest,
    ) -> SlackApiResponse<UserLookupResponse>;

    async fn list_members(
        &self,
        request: &ListMembersRequestParams,
    ) -> SlackApiResponse<ListMembersResponse>;
    async fn list_scheduled_messages(
        &self,
        request: &ScheduledMessagesListRequest,
    ) -> SlackApiResponse<ScheduledMessagesListRaw>;
    async fn delete_scheduled_message(
        &self,
        request: &DeleteScheduledMessageRequest,
    ) -> SlackApiResponse<Empty>;
    async fn auth_test(&self) -> SlackApiResponse<BotIdentity>;
}

pub(crate) struct ProdSlackApiClient {
    client: Client,
}
impl ProdSlackApiClient {
    pub fn new(token: String) -> ProdSlackApiClient {
        ProdSlackApiClient {
            client: create_client(token),
        }
    }
}

#[async_trait(?Send)]
impl SlackApiClient for ProdSlackApiClient {
    async fn schedule_message(&self, request: &ScheduleMessageRequest) -> ScheduleMessageResponse {
        let endpoint = ScheduleMessageEndpoint;
        let response_raw = endpoint.call_endpoint(request, &self.client).await;
        ScheduleMessageResponse::from(response_raw.unwrap())
    }

    async fn join_conversation(
        &self,
        request: &JoinConversationRequest,
    ) -> SlackApiResponse<JoinConversationResponse> {
        let endpoint = JoinConversationEndpoint;
        endpoint.call_endpoint(request, &self.client).await
    }

    async fn user_lookup_by_email(
        &self,
        request: &UserLookupRequest,
    ) -> SlackApiResponse<UserLookupResponse> {
        let endpoint = UserLookupByEmailEndpoint;
        endpoint.call_endpoint(request, &self.client).await
    }

    async fn list_members(
        &self,
        request: &ListMembersRequestParams,
    ) -> SlackApiResponse<ListMembersResponse> {
        let endpoint = ListMembersEndpoint;
        endpoint.call_endpoint(request, &self.client).await
    }

    async fn list_scheduled_messages(
        &self,
        request: &ScheduledMessagesListRequest,
    ) -> SlackApiResponse<ScheduledMessagesListRaw> {
        let endpoint = ListScheduledMessagesEndpoint;
        endpoint.call_endpoint(request, &self.client).await
    }

    async fn delete_scheduled_message(
        &self,
        request: &DeleteScheduledMessageRequest,
    ) -> SlackApiResponse<Empty> {
        let endpoint = DeleteScheduledMessageEndpoint;
        endpoint.call_endpoint(request, &self.client).await
    }

    async fn auth_test(&self) -> SlackApiResponse<BotIdentity> {
        let endpoint = AuthTestEndpoint;

        endpoint.call_endpoint(&Empty {}, &self.client).await
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::from_str;

    pub struct TestSlackClient {
        schedule_message_res: String,
        join_conversation_res: String,
        user_lookup_res: String,
        list_members_res: String,
        list_scheduled_messages_res: String,
        delete_scheduled_message_res: String,
        auth_test_res: String,
    }

    #[async_trait(?Send)]
    impl SlackApiClient for TestSlackClient {
        async fn schedule_message(
            &self,
            _request: &ScheduleMessageRequest,
        ) -> ScheduleMessageResponse {
            from_str(&self.schedule_message_res).unwrap()
        }

        async fn join_conversation(
            &self,
            _request: &JoinConversationRequest,
        ) -> SlackApiResponse<JoinConversationResponse> {
            from_str(&self.join_conversation_res).unwrap()
        }

        async fn user_lookup_by_email(
            &self,
            _request: &UserLookupRequest,
        ) -> SlackApiResponse<UserLookupResponse> {
            from_str(&self.user_lookup_res).unwrap()
        }

        async fn list_members(
            &self,
            _request: &ListMembersRequestParams,
        ) -> SlackApiResponse<ListMembersResponse> {
            from_str(&self.list_members_res).unwrap()
        }

        async fn list_scheduled_messages(
            &self,
            _request: &ScheduledMessagesListRequest,
        ) -> SlackApiResponse<ScheduledMessagesListRaw> {
            from_str(&self.list_scheduled_messages_res).unwrap()
        }

        async fn delete_scheduled_message(
            &self,
            _request: &DeleteScheduledMessageRequest,
        ) -> SlackApiResponse<Empty> {
            from_str(&self.delete_scheduled_message_res).unwrap()
        }

        async fn auth_test(&self) -> SlackApiResponse<BotIdentity> {
            from_str(&self.auth_test_res).unwrap()
        }
    }

    impl Default for TestSlackClient {
        fn default() -> Self {
            let schedule_message_res = r#"{
                "ok": true,
                "channel": "C1H9RESGL",
                "scheduled_message_id": "Q1298393284",
                "post_at": 1562180400,
                "message": {
                    "text": "Here's a message for you in the future",
                    "username": "ecto1",
                    "bot_id": "B19LU7CSY",
                    "attachments": [
                        {
                            "text": "This is an attachment",
                            "id": 1,
                            "fallback": "This is an attachment's fallback"
                        }
                    ],
                    "type": "delayed_message",
                    "subtype": "bot_message"
                }
            }"#
            .to_string();
            let join_conversation_res = r#"{
                "ok": true,
                "channel": {
                    "id": "C061EG9SL",
                    "name": "general",
                    "is_channel": true,
                    "is_group": false,
                    "is_im": false,
                    "created": 1449252889,
                    "creator": "U061F7AUR",
                    "is_archived": false,
                    "is_general": true,
                    "unlinked": 0,
                    "name_normalized": "general",
                    "is_shared": false,
                    "is_ext_shared": false,
                    "is_org_shared": false,
                    "pending_shared": [],
                    "is_pending_ext_shared": false,
                    "is_member": true,
                    "is_private": false,
                    "is_mpim": false,
                    "topic": {
                        "value": "Which widget do you worry about?",
                        "creator": "",
                        "last_set": 0
                    },
                    "purpose": {
                        "value": "For widget discussion",
                        "creator": "",
                        "last_set": 0
                    },
                    "previous_names": []
                },
                "warning": "already_in_channel",
                "response_metadata": {
                    "warnings": [
                        "already_in_channel"
                    ]
                }
            }"#
            .to_string();
            let user_lookup_res = r#"{
                "ok": true,
                "user": {
                    "id": "W012A3CDE",
                    "team_id": "T012AB3C4",
                    "name": "spengler",
                    "deleted": false,
                    "color": "9f69e7",
                    "real_name": "Egon Spengler",
                    "tz": "America/New_York",
                    "tz_label": "Eastern Daylight Time",
                    "tz_offset": -14400,
                    "profile": {
                        "title": "",
                        "phone": "",
                        "skype": "",
                        "real_name": "Egon Spengler",
                        "real_name_normalized": "Egon Spengler",
                        "display_name": "spengler",
                        "display_name_normalized": "spengler",
                        "status_text": "Print is dead",
                        "status_emoji": ":books:",
                        "status_expiration": 1502138999,
                        "avatar_hash": "ge3b51ca72de",
                        "first_name": "Matthew",
                        "last_name": "Johnston",
                        "email": "spengler@ghostbusters.example.com",
                        "image_original": "https://.../avatar/e3b51ca72dee4ef87916ae2b9240df50.jpg",
                        "image_24": "https://.../avatar/e3b51ca72dee4ef87916ae2b9240df50.jpg",
                        "image_32": "https://.../avatar/e3b51ca72dee4ef87916ae2b9240df50.jpg",
                        "image_48": "https://.../avatar/e3b51ca72dee4ef87916ae2b9240df50.jpg",
                        "image_72": "https://.../avatar/e3b51ca72dee4ef87916ae2b9240df50.jpg",
                        "image_192": "https://.../avatar/e3b51ca72dee4ef87916ae2b9240df50.jpg",
                        "image_512": "https://.../avatar/e3b51ca72dee4ef87916ae2b9240df50.jpg",
                        "team": "T012AB3C4"
                    },
                    "is_admin": true,
                    "is_owner": false,
                    "is_primary_owner": false,
                    "is_restricted": false,
                    "is_ultra_restricted": false,
                    "is_bot": false,
                    "is_stranger": false,
                    "updated": 1502138686,
                    "is_app_user": false,
                    "is_invited_user": false,
                    "has_2fa": false,
                    "locale": "en-US"
                }
            }"#
            .to_string();
            let list_members_res = r#"{
                "ok": true,
                "members": [
                    "U023BECGF",
                    "U061F7AUR",
                    "W012A3CDE",
                    "W012A3CDA"
                ],
                "response_metadata": {
                    "next_cursor": ""
                }
            }"#
            .to_string();
            let list_scheduled_messages_res = r#"{
                "ok": true,
                "scheduled_messages": [
                    {
                        "id": "1298393284",
                        "channel_id": "C1H9RESGL",
                        "post_at": 1606965300,
                        "date_created": 1551891734,
                        "text": "Here's a  final message for you in the future"
                    }
                ],
                "response_metadata": {
                    "next_cursor": ""
                }
            }"#
            .to_string();
            let delete_scheduled_message_res = r#"{
                "ok": true
            }"#
            .to_string();
            let auth_test_res = r#"{
                "ok": true,
                "url": "https://subarachnoid.slack.com/",
                "team": "Subarachnoid Workspace",
                "user": "grace",
                "team_id": "T12345678",
                "user_id": "W12345678",
                "bot_id": "W12345678"
            }"#
            .to_string();

            TestSlackClient {
                schedule_message_res,
                join_conversation_res,
                user_lookup_res,
                list_members_res,
                list_scheduled_messages_res,
                delete_scheduled_message_res,
                auth_test_res,
            }
        }
    }
}
