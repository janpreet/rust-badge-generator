mod error;

use error::BadgeError;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT, CONTENT_TYPE};
use serde_json::{json, Value};
use std::env;
use std::fs::File;
use std::io::Write;

#[cfg(test)]
use mockito::{mock, Matcher, Server};

async fn fetch_github_stats_with_url(owner: &str, repo: &str, package: Option<&str>, url: &str) -> Result<u64, BadgeError> {
    let github_token = env::var("GITHUB_TOKEN")?;

    println!("Fetching stats for GitHub package: {}/{}/{:?}", owner, repo, package);

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", github_token))
        .map_err(|e| BadgeError::InvalidHeader(e.to_string()))?);
    headers.insert(USER_AGENT, HeaderValue::from_static("rust-reqwest"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let query = match package {
        Some(pkg) => format!(
            r#"{{
                repository(owner: "{}", name: "{}") {{
                    packages(first: 1, names: "{}") {{
                        nodes {{
                            name
                            statistics {{
                                downloadsTotalCount
                            }}
                        }}
                    }}
                }}
            }}"#,
            owner, repo, pkg
        ),
        None => format!(
            r#"{{
                repository(owner: "{}", name: "{}") {{
                    releases(last: 1) {{
                        totalCount
                    }}
                }}
            }}"#,
            owner, repo
        ),
    };

    let body = json!({
        "query": query
    });

    let response = client.post(url)
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    println!("Response status: {}", response.status());

    if !response.status().is_success() {
        return Err(BadgeError::NetworkError(format!("HTTP error: {}", response.status())));
    }

    let response_body = response.text().await?;
    println!("Response body: {}", response_body);

    if response_body.trim().is_empty() {
        return Err(BadgeError::NoDownloads);
    }

    let data: Value = serde_json::from_str(&response_body)?;

    if let Some(error) = data.get("errors") {
        println!("GraphQL Error: {:?}", error);
        return Err(BadgeError::NoDownloads);
    }

    let downloads = match package {
        Some(_) => data["data"]["repository"]["packages"]["nodes"]
            .as_array()
            .and_then(|nodes| nodes.get(0))
            .and_then(|node| node["statistics"]["downloadsTotalCount"].as_u64()),
        None => data["data"]["repository"]["releases"]["totalCount"].as_u64(),
    }.ok_or_else(|| {
        println!("No download data found for package: {}/{}/{:?}", owner, repo, package);
        BadgeError::NoDownloads
    })?;

    Ok(downloads)
}

async fn fetch_github_stats(owner: &str, repo: &str, package: Option<&str>) -> Result<u64, BadgeError> {
    fetch_github_stats_with_url(owner, repo, package, "https://api.github.com/graphql").await
}

async fn fetch_dockerhub_stats(owner: &str, repo: &str) -> Result<u64, BadgeError> {
    let url = format!(
        "https://hub.docker.com/v2/repositories/{}/{}/",
        owner, repo
    );

    println!("Fetching stats for DockerHub image: {}/{}", owner, repo);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    println!("Response status: {}", response.status());

    let data: Value = response.json().await?;
    println!("Response body: {}", serde_json::to_string_pretty(&data)?);

    let pull_count = data["pull_count"].as_u64().ok_or(BadgeError::NoDownloads)?;

    Ok(pull_count)
}

async fn fetch_npm_stats(package: &str) -> Result<u64, BadgeError> {
    let url = format!(
        "https://api.npmjs.org/downloads/point/last-month/{}",
        package
    );

    println!("Fetching stats for NPM package: {}", package);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    println!("Response status: {}", response.status());

    let data: Value = response.json().await?;
    println!("Response body: {}", serde_json::to_string_pretty(&data)?);

    let downloads = data["downloads"].as_u64().ok_or(BadgeError::NoDownloads)?;

    Ok(downloads)
}

fn generate_badge(label: &str, message: &str, color: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="96" height="20" role="img" aria-label="{label}: {message}">
  <title>{label}: {message}</title>
  <linearGradient id="s" x2="0" y2="100%">
    <stop offset="0" stop-color="#bbb" stop-opacity=".1"/>
    <stop offset="1" stop-opacity=".1"/>
  </linearGradient>
  <clipPath id="r">
    <rect width="96" height="20" rx="3" fill="#fff"/>
  </clipPath>
  <g clip-path="url(#r)">
    <rect width="61" height="20" fill="#555"/>
    <rect x="61" width="35" height="20" fill="{color}"/>
    <rect width="96" height="20" fill="url(#s)"/>
  </g>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" text-rendering="geometricPrecision" font-size="110">
    <text aria-hidden="true" x="315" y="150" fill="#010101" fill-opacity=".3" transform="scale(.1)" textLength="510">{label}</text>
    <text x="315" y="140" transform="scale(.1)" fill="#fff" textLength="510">{label}</text>
    <text aria-hidden="true" x="775" y="150" fill="#010101" fill-opacity=".3" transform="scale(.1)" textLength="250">{message}</text>
    <text x="775" y="140" transform="scale(.1)" fill="#fff" textLength="250">{message}</text>
  </g>
</svg>"##,
        label = label,
        message = message,
        color = color
    )
}

#[tokio::main]
async fn main() -> Result<(), BadgeError> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <registry> <owner> <repo> [package]", args[0]);
        std::process::exit(1);
    }

    let registry = &args[1];
    let owner = &args[2];
    let repo = &args[3];
    let package = args.get(4).map(|s| s.as_str());

    println!("Fetching stats for: {} {}/{} {:?}", registry, owner, repo, package);

    let downloads = match registry.as_str() {
        "github" => fetch_github_stats(owner, repo, package).await?,
        "dockerhub" => fetch_dockerhub_stats(owner, repo).await?,
        "npm" => fetch_npm_stats(package.unwrap()).await?,
        unknown => return Err(BadgeError::UnknownRegistry(unknown.to_string())),
    };

    println!("Downloads: {}", downloads);

    let badge_svg = generate_badge("downloads", &downloads.to_string(), "#007ec6");
    let filename = format!("badges/{}-{}-{}-downloads.svg", owner, repo, package.unwrap_or("unknown"));
    let mut file = File::create(&filename)?;
    file.write_all(badge_svg.as_bytes())?;

    println!("Badge generated successfully: {}", filename);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_github_stats_success() {
        let mock_server = Server::new();
        let mock = mock_server.mock("POST", "/graphql")
            .match_header("authorization", "Bearer test_token")
            .match_body(Matcher::Json(json!({
                "query": r#"{
                    repository(owner: "test_owner", name: "test_repo") {
                        packages(first: 1, names: "test-package") {
                            nodes {
                                name
                                statistics {
                                    downloadsTotalCount
                                }
                            }
                        }
                    }
                }"#
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"
            {
                "data": {
                    "repository": {
                        "packages": {
                            "nodes": [
                                {
                                    "name": "test-package",
                                    "statistics": {
                                        "downloadsTotalCount": 42
                                    }
                                }
                            ]
                        }
                    }
                }
            }"#)
            .create();

        std::env::set_var("GITHUB_TOKEN", "test_token");
        
        let url = mock_server.url();
        let result = fetch_github_stats_with_url("test_owner", "test_repo", Some("test-package"), &url).await;
        
        mock.assert();
        assert!(result.is_ok(), "Error: {:?}", result.err());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_fetch_github_stats_no_downloads() {
        let mock_server = Server::new();
        let mock = mock_server.mock("POST", "/graphql")
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"
            {
                "data": {
                    "repository": {
                        "packages": {
                            "nodes": []
                        }
                    }
                }
            }"#)
            .create();

        std::env::set_var("GITHUB_TOKEN", "test_token");
        let result = fetch_github_stats_with_url("test_owner", "test_repo", Some("test-package"), &mock_server.url()).await;
        mock.assert();
        assert!(matches!(result, Err(BadgeError::NoDownloads)));
    }

    #[tokio::test]
    async fn test_fetch_github_stats_no_package() {
        let mock_server = Server::new();
        let mock = mock_server.mock("POST", "/graphql")
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"
            {
                "data": {
                    "repository": {
                        "releases": {
                            "totalCount": 10
                        }
                    }
                }
            }"#)
            .create();

        std::env::set_var("GITHUB_TOKEN", "test_token");
        let result = fetch_github_stats_with_url("test_owner", "test_repo", None, &mock_server.url()).await;
        mock.assert();
        assert!(result.is_ok(), "Error: {:?}", result.err());
        assert_eq!(result.unwrap(), 10);
    }

    #[tokio::test]
    async fn test_fetch_github_stats_empty_response() {
        let mock_server = Server::new();
        let mock = mock_server.mock("POST", "/graphql")
            .match_header("authorization", "Bearer test_token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("")
            .create();

        std::env::set_var("GITHUB_TOKEN", "test_token");
        let result = fetch_github_stats_with_url("test_owner", "test_repo", Some("test-package"), &mock_server.url()).await;
        mock.assert();
        assert!(matches!(result, Err(BadgeError::NoDownloads)));
    }

    #[test]
    fn test_generate_badge() {
        let badge = generate_badge("downloads", "42", "#007ec6");
        assert!(badge.contains("downloads"));
        assert!(badge.contains("42"));
        assert!(badge.contains("#007ec6"));
    }
}