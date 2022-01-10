use super::*;
use crate::SLACK_API_URL;
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
