use std::{fmt::Display, ops::ControlFlow};

use anyhow::anyhow;
use reqwest::{
    header::{HeaderName, HeaderValue},
    Response,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::time::{self, Duration, MissedTickBehavior};

const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Deserialize)]
struct GitHubError {
    message: String,
}

impl Display for GitHubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GitHub error: ")?;
        f.write_str(&self.message)?;

        Ok(())
    }
}

#[derive(Deserialize)]
struct Issue {
    number: u64,
}

#[derive(Serialize)]
struct CreateIssue {
    title: &'static str,
    body: String,
}

// const API_URL: &str = "https://api.github.com/repos/Mathspy/rust-is-beautiful/issues";
const API_URL: &str = "https://api.github.com/repos/rust-lang/rust/issues";

async fn get_response_data<T>(response: reqwest::Result<Response>) -> Option<T>
where
    T: DeserializeOwned,
{
    match response {
        Ok(response) if response.status().is_success() => match response.json::<T>().await {
            Ok(issues) => Some(issues),
            Err(error) => {
                println!("Unexpected error while decoding GitHub response {error}");
                None
            }
        },
        Ok(response) => {
            match response.json::<GitHubError>().await {
                Ok(error) => println!("{error}"),
                Err(error) => println!("Unexpected error while decoding GitHub error {error}"),
            };
            None
        }
        Err(error) => {
            println!("Unexpected error while making request {error}");
            None
        }
    }
}

async fn attempt(
    client: &reqwest::Client,
    magic_number: u64,
) -> ControlFlow<Result<(), anyhow::Error>, Option<anyhow::Error>> {
    let response = client
        .get(API_URL)
        .query(&[("per_page", "1"), ("state", "all")])
        .send()
        .await;

    let issues = if let Some(issues) = get_response_data::<Vec<Issue>>(response).await {
        issues
    } else {
        return ControlFlow::Continue(None);
    };

    let issue = match issues.get(0) {
        None => {
            println!("GitHub returned 0 issues for some reason ??");
            return ControlFlow::Continue(None);
        }
        Some(issue) => issue,
    };

    match magic_number.cmp(&(issue.number + 1)) {
        std::cmp::Ordering::Less => {
            return ControlFlow::Break(Err(anyhow!("We didn't make it, I am sorry :<")));
        }
        std::cmp::Ordering::Equal => {}
        std::cmp::Ordering::Greater => {
            return ControlFlow::Continue(None);
        }
    };

    let posted_issue = if let Some(posted_issue) = send_request(client).await {
        posted_issue
    } else {
        return ControlFlow::Break(Err(anyhow!("We failed to post the issue, noooo")));
    };

    if posted_issue.number == magic_number {
        ControlFlow::Break(Ok(()))
    } else {
        ControlFlow::Break(Err(anyhow!("Oh no we missed it ahhhh!!")))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env variable missing");
    let magic_number = std::env::var("MAGIC_NUMBER")
        .expect("MAGIC_NUMBER env variable missing")
        .parse::<u64>()
        .expect("MAGIC_NUMBER to be a number");

    let mut interval = time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let headers = [
        (
            HeaderName::from_static("accept"),
            HeaderValue::from_static("application/vnd.github+json"),
        ),
        (
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("token {token}"))
                .expect("GITHUB_TOKEN to be a valid header"),
        ),
    ];
    let headers_map = headers.into_iter().collect();

    let client = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .default_headers(headers_map)
        .build()
        .expect("failed to build client");

    println!("Rust is Beautiful ❤️");

    loop {
        interval.tick().await;

        match attempt(&client, magic_number).await {
            ControlFlow::Continue(None) => continue,
            ControlFlow::Continue(Some(error)) => {
                println!("Encountered soft error: {error:?}");
                continue;
            }
            ControlFlow::Break(Ok(())) => {
                println!("We did it!");
                break;
            }
            ControlFlow::Break(Err(error)) => {
                println!("Terminating with error: {error:?}");
                break;
            }
        }
    }
}

async fn send_request(client: &reqwest::Client) -> Option<Issue> {
    let file = tokio::fs::read_to_string("assets/issue.md")
        .await
        .map_or_else(
            |err| {
                println!("Failed to read issue.md file {err}");
                None
            },
            |file| Some(file),
        )?;

    let issue = CreateIssue {
        title: "Rust is Beautiful",
        body: file,
    };

    let response = client.post(API_URL).json(&issue).send().await;

    get_response_data::<Issue>(response).await
}
