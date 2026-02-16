use anyhow::{Context, Result};
use clap::Parser;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::ExitCode;

const DEFAULT_API_URL: &str = "https://api.frankwiles.com/api/storage/create/";

#[derive(Parser, Debug)]
#[command(name = "store")]
#[command(about = "Store data in the Frank Wiles API", long_about = None)]
struct Args {
    /// Data to store: either a JSON string or key=value pairs
    #[arg(required = true, trailing_var_arg = true)]
    data: Vec<String>,

    /// API token (or set STORE_API_TOKEN env var)
    #[arg(long, env = "STORE_API_TOKEN")]
    api_token: String,

    /// Project slug (or set STORE_PROJECT env var)
    #[arg(long, env = "STORE_PROJECT")]
    project: String,

    /// API URL (or set STORE_API_URL env var)
    #[arg(long, env = "STORE_API_URL", default_value = DEFAULT_API_URL)]
    api_url: String,

    /// Data type categorization (optional)
    #[arg(long)]
    r#type: Option<String>,
}

#[derive(Serialize)]
struct Payload {
    project_slug: String,
    data_type: Option<String>,
    data: serde_json::Value,
}

#[derive(Deserialize)]
struct ApiError {
    detail: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

fn parse_data_input(inputs: &[String]) -> Result<serde_json::Value> {
    // Single argument: try to parse as JSON
    if inputs.len() == 1 {
        let input = &inputs[0];
        if let Ok(json) = serde_json::from_str(input) {
            return Ok(json);
        }
    }

    // Multiple arguments or failed JSON parse: treat as key=value pairs
    let mut map = HashMap::new();
    for pair in inputs {
        let (key, value) = pair
            .split_once('=')
            .with_context(|| format!("Invalid key=value pair: '{}'. Expected format: key=value", pair))?;

        // Try to parse value as JSON, otherwise treat as string
        let json_value: serde_json::Value = if let Ok(v) = serde_json::from_str(value) {
            v
        } else {
            serde_json::Value::String(value.to_string())
        };

        map.insert(key.to_string(), json_value);
    }

    Ok(serde_json::Value::Object(
        map.into_iter()
            .collect(),
    ))
}

#[tokio::main]
async fn main() -> ExitCode {
    // Parse args first, letting clap handle --help and --version
    let args = match Args::try_parse() {
        Ok(a) => a,
        Err(e) => {
            // Clap handles its own display for --help/--version
            e.print().ok();
            return if e.exit_code() == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
        }
    };

    match run(args).await {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("\x1b[31mError:\x1b[0m {}", e);

            // Add chain context if available
            let mut source = e.source();
            while let Some(cause) = source {
                eprintln!("\x1b[31m  Caused by:\x1b[0m {}", cause);
                source = cause.source();
            }

            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<()> {
    let data = parse_data_input(&args.data)
        .context("Failed to parse data input")?;

    let payload = Payload {
        project_slug: args.project.clone(),
        data_type: args.r#type,
        data,
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&args.api_url)
        .bearer_auth(&args.api_token)
        .json(&payload)
        .send()
        .await
        .context("Failed to send request to API")?;

    let status = response.status();

    if status.is_success() {
        let body = response.text().await.unwrap_or_default();
        println!("\x1b[32mSuccess:\x1b[0m Data stored successfully");
        if !body.is_empty() && body != "null" {
            println!("{}", body);
        }
        Ok(())
    } else {
        let error_body = response.text().await.unwrap_or_default();

        // Try to parse API error response
        let error_msg = if let Ok(api_error) = serde_json::from_str::<ApiError>(&error_body) {
            api_error.detail.or(api_error.message).unwrap_or(error_body)
        } else {
            error_body
        };

        let friendly_status = match status {
            StatusCode::UNAUTHORIZED => "Unauthorized - check your API token".to_string(),
            StatusCode::FORBIDDEN => "Forbidden - you don't have permission for this project".to_string(),
            StatusCode::NOT_FOUND => "Not found - check the API URL and project slug".to_string(),
            StatusCode::BAD_REQUEST => format!("Bad request - {}", error_msg),
            StatusCode::INTERNAL_SERVER_ERROR => "Server error - please try again later".to_string(),
            _ => format!("HTTP {} - {}", status.as_u16(), error_msg),
        };

        anyhow::bail!("API request failed: {}", friendly_status)
    }
}
