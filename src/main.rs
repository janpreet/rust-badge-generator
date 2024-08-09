use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT, CONTENT_TYPE};
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;

async fn fetch_github_stats(owner: &str, _repo: &str, package: &str) -> Result<u64, Box<dyn Error>> {
    let github_token = env::var("GITHUB_TOKEN")?;
    let url = "https://api.github.com/graphql";

    println!("Fetching stats from GraphQL API");

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", github_token))?);
    headers.insert(USER_AGENT, HeaderValue::from_static("rust-reqwest"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let query = format!(
        r#"{{
            user(login: "{}") {{
                packages(first: 1, names: "{}") {{
                    nodes {{
                        statistics {{
                            downloadsTotalCount
                        }}
                    }}
                }}
            }}
        }}"#,
        owner, package
    );

    let body = json!({
        "query": query
    });

    let response = client.post(url)
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    println!("Response status: {}", response.status());

    let response_body = response.text().await?;
    println!("Response body: {}", response_body);

    let data: Value = serde_json::from_str(&response_body)?;
    
    let downloads = data["data"]["user"]["packages"]["nodes"][0]["statistics"]["downloadsTotalCount"]
        .as_u64()
        .unwrap_or(0);

    println!("Parsed downloads: {}", downloads);
    Ok(downloads)
}

async fn fetch_dockerhub_stats(owner: &str, repo: &str) -> Result<u64, Box<dyn Error>> {
    let url = format!(
        "https://hub.docker.com/v2/repositories/{}/{}/",
        owner, repo
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let data: Value = response.json().await?;
    Ok(data["pull_count"].as_u64().unwrap_or(0))
}

async fn fetch_npm_stats(package: &str) -> Result<u64, Box<dyn Error>> {
    let url = format!(
        "https://api.npmjs.org/downloads/point/last-month/{}",
        package
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let data: Value = response.json().await?;
    Ok(data["downloads"].as_u64().unwrap_or(0))
}

fn generate_badge(label: &str, message: &str, color: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
        <svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"96\" height=\"20\" role=\"img\" aria-label=\"{label}: {message}\">\
          <title>{label}: {message}</title>\
          <linearGradient id=\"s\" x2=\"0\" y2=\"100%\">\
            <stop offset=\"0\" stop-color=\"#bbb\" stop-opacity=\".1\"/>\
            <stop offset=\"1\" stop-opacity=\".1\"/>\
          </linearGradient>\
          <clipPath id=\"r\">\
            <rect width=\"96\" height=\"20\" rx=\"3\" fill=\"#fff\"/>\
          </clipPath>\
          <g clip-path=\"url(#r)\">\
            <rect width=\"61\" height=\"20\" fill=\"#555\"/>\
            <rect x=\"61\" width=\"35\" height=\"20\" fill=\"{color}\"/>\
            <rect width=\"96\" height=\"20\" fill=\"url(#s)\"/>\
          </g>\
          <g fill=\"#fff\" text-anchor=\"middle\" font-family=\"Verdana,Geneva,DejaVu Sans,sans-serif\" text-rendering=\"geometricPrecision\" font-size=\"110\">\
            <text aria-hidden=\"true\" x=\"315\" y=\"150\" fill=\"#010101\" fill-opacity=\".3\" transform=\"scale(.1)\" textLength=\"510\">{label}</text>\
            <text x=\"315\" y=\"140\" transform=\"scale(.1)\" fill=\"#fff\" textLength=\"510\">{label}</text>\
            <text aria-hidden=\"true\" x=\"775\" y=\"150\" fill=\"#010101\" fill-opacity=\".3\" transform=\"scale(.1)\" textLength=\"250\">{message}</text>\
            <text x=\"775\" y=\"140\" transform=\"scale(.1)\" fill=\"#fff\" textLength=\"250\">{message}</text>\
          </g>\
        </svg>",
        label = label,
        message = message,
        color = color
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        eprintln!("Usage: {} <registry> <owner> <repo> <package>", args[0]);
        std::process::exit(1);
    }

    let registry = &args[1];
    let owner = &args[2];
    let repo = &args[3];
    let package = &args[4];

    let downloads = match registry.as_str() {
        "github" => fetch_github_stats(owner, repo, package).await?,
        "dockerhub" => fetch_dockerhub_stats(owner, repo).await?,
        "npm" => fetch_npm_stats(package).await?,
        _ => {
            eprintln!("Unsupported registry: {}", registry);
            std::process::exit(1);
        }
    };

    let badge_svg = generate_badge("downloads", &downloads.to_string(), "#007ec6");
    let filename = format!("badges/{}-{}-{}-downloads.svg", owner, repo, package);
    let mut file = File::create(filename)?;
    file.write_all(badge_svg.as_bytes())?;

    println!("Badge generated successfully");
    Ok(())
}