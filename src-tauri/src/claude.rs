use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsageData {
    pub messages_used: Option<u32>,
    pub messages_limit: Option<u32>,
    pub reset_at: Option<String>,
    pub plan: Option<String>,
    pub org_name: Option<String>,
    pub email: Option<String>,
    pub fetched_at: String,
    pub source: String,
}

impl UsageData {
    pub fn usage_percent(&self) -> Option<f64> {
        match (self.messages_used, self.messages_limit) {
            (Some(used), Some(limit)) if limit > 0 => Some(used as f64 / limit as f64 * 100.0),
            _ => None,
        }
    }
}

fn claude_headers(session_key: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("sessionKey={session_key}")).unwrap(),
    );
    headers.insert(USER_AGENT, HeaderValue::from_static(UA));
    headers.insert(REFERER, HeaderValue::from_static("https://claude.ai/"));
    headers.insert("anthropic-client-platform", HeaderValue::from_static("web_claude_ai"));
    headers
}

pub async fn fetch_claude_usage(session_key: &str) -> Result<UsageData, String> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| e.to_string())?;

    // Step 1: bootstrap — get org UUID and account info
    let bootstrap: serde_json::Value = client
        .get("https://claude.ai/api/bootstrap")
        .headers(claude_headers(session_key))
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Parse error: {e}"))?;

    let email = bootstrap
        .pointer("/account/email")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let org = bootstrap
        .pointer("/account/memberships/0/organization")
        .or_else(|| bootstrap.pointer("/organizations/0"));

    let org_name = org
        .and_then(|o| o.get("name"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let org_uuid = org
        .and_then(|o| o.get("uuid"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let plan = org
        .and_then(|o| o.pointer("/subscription/plans/0/name"))
        .or_else(|| org.and_then(|o| o.pointer("/plan/name")))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    // Step 2: try the usage endpoint
    let mut messages_used = None;
    let mut messages_limit = None;
    let mut reset_at = None;

    if let Some(uuid) = &org_uuid {
        if let Ok(resp) = client
            .get(format!("https://claude.ai/api/organizations/{uuid}/usage"))
            .headers(claude_headers(session_key))
            .send()
            .await
        {
            if let Ok(usage) = resp.json::<serde_json::Value>().await {
                // Try common field shapes
                let root = &usage;
                messages_used = extract_u32(root, &[
                    "/message_limit/used",
                    "/messages_used",
                    "/used",
                    "/prompts_used",
                ]);
                messages_limit = extract_u32(root, &[
                    "/message_limit/max",
                    "/messages_limit",
                    "/max",
                    "/limit",
                ]);
                reset_at = extract_str(root, &[
                    "/message_limit/reset_at",
                    "/reset_at",
                    "/resets_at",
                ]);
            }
        }

        // Fallback: try rate_limit_status endpoint
        if messages_limit.is_none() {
            if let Ok(resp) = client
                .get(format!(
                    "https://claude.ai/api/organizations/{uuid}/rate_limit_status"
                ))
                .headers(claude_headers(session_key))
                .send()
                .await
            {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    messages_used = messages_used.or_else(|| extract_u32(&data, &["/used", "/prompts_used"]));
                    messages_limit = messages_limit.or_else(|| extract_u32(&data, &["/limit", "/max"]));
                    reset_at = reset_at.or_else(|| extract_str(&data, &["/reset_at", "/resets_at"]));
                }
            }
        }
    }

    Ok(UsageData {
        messages_used,
        messages_limit,
        reset_at,
        plan,
        org_name,
        email,
        fetched_at: chrono::Utc::now().to_rfc3339(),
        source: "claude_ai".to_string(),
    })
}

pub async fn verify_api_key(api_key: &str) -> Result<UsageData, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    if resp.status().is_success() {
        Ok(UsageData {
            messages_used: None,
            messages_limit: None,
            reset_at: None,
            plan: Some("API".to_string()),
            org_name: None,
            email: None,
            fetched_at: chrono::Utc::now().to_rfc3339(),
            source: "api_key".to_string(),
        })
    } else {
        Err(format!("Invalid API key ({})", resp.status()))
    }
}

fn extract_u32(v: &serde_json::Value, paths: &[&str]) -> Option<u32> {
    for path in paths {
        if let Some(val) = v.pointer(path) {
            if let Some(n) = val.as_u64() {
                return Some(n as u32);
            }
        }
    }
    None
}

fn extract_str(v: &serde_json::Value, paths: &[&str]) -> Option<String> {
    for path in paths {
        if let Some(val) = v.pointer(path) {
            if let Some(s) = val.as_str() {
                return Some(s.to_string());
            }
        }
    }
    None
}
