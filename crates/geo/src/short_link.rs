//! The address a short link points to. Only the redirect is read, never a page.

use std::{fmt, time::Duration};

#[derive(Debug)]
pub enum RedirectError {
    Request(reqwest::Error),
    NotARedirect,
}

impl fmt::Display for RedirectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(error) => error.fmt(formatter),
            Self::NotARedirect => formatter.write_str("the link leads nowhere"),
        }
    }
}

impl std::error::Error for RedirectError {}

fn failed(error: reqwest::Error) -> RedirectError {
    // The link itself stays out of the message.
    RedirectError::Request(error.without_url())
}

/// Asks for `url` once and returns where it sends the caller.
pub async fn redirect_target(url: &str, timeout: Duration) -> Result<String, RedirectError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .build()
        .map_err(failed)?;
    let response = client.get(url).send().await.map_err(failed)?;
    if !response.status().is_redirection() {
        return Err(RedirectError::NotARedirect);
    }
    response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
        .ok_or(RedirectError::NotARedirect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, response::Redirect, routing::get};

    #[tokio::test]
    async fn only_the_address_of_the_redirect_is_returned() {
        let app = Router::new()
            .route(
                "/short",
                get(|| async { Redirect::temporary("https://example.invalid/long?x=1") }),
            )
            .route("/page", get(|| async { "a page" }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let base = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move { axum::serve(listener, app).await });
        let timeout = Duration::from_secs(5);

        let target = redirect_target(&format!("{base}/short"), timeout)
            .await
            .expect("redirect");
        assert_eq!(target, "https://example.invalid/long?x=1");
        assert!(matches!(
            redirect_target(&format!("{base}/page"), timeout).await,
            Err(RedirectError::NotARedirect)
        ));
        server.abort();
    }
}
