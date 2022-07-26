use std::fmt::Display;

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

    loop {
        interval.tick().await;

        let response = client
            .get(API_URL)
            .query(&[("per_page", "1"), ("state", "all")])
            .send()
            .await;

        let issues = if let Some(issues) = get_response_data::<Vec<Issue>>(response).await {
            issues
        } else {
            continue;
        };

        let issue = match issues.get(0) {
            None => {
                println!("GitHub returned 0 issues for some reason ??");
                continue;
            }
            Some(issue) => issue,
        };

        match magic_number.cmp(&(issue.number + 1)) {
            std::cmp::Ordering::Less => {
                println!("We didn't make it, I am sorry :<");
                break;
            }
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => {
                continue;
            }
        };

        let posted_issue = if let Some(posted_issue) = send_request(client).await {
            posted_issue
        } else {
            println!("We failed to post the issue, noooo");
            break;
        };

        if posted_issue.number == magic_number {
            println!("We did it!");
        } else {
            println!("Oh no we missed it ahhhh!!");
        }
        break;
    }
}

async fn send_request(client: reqwest::Client) -> Option<Issue> {
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
