use super::*;

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
        req.insert_header(surf::http::headers::CONTENT_TYPE, format!("{}; charset=utf-8", surf::http::mime::JSON));
        let res = next.run(req, client).await?;
        Ok(res)
    }
}

pub(crate) fn yes() -> bool {
    let mut buff = String::new();
    match std::io::stdin().read_line(&mut buff) {
        Ok(_bytes) => {
            if buff.to_ascii_lowercase().trim() == "y".to_string() {
                true
            } else { false }
        }
        Err(_err) => { false },
    }
}

pub(crate) fn create_client(token: String) -> Client {
    let headers = HeadersMiddleware { token };
    let mut client = Client::new()
        .with(Logger::new())
        .with(headers);
    client.set_base_url(Url::parse(SLACK_API_URL).unwrap());
    client
}