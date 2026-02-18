use semver::Version;
use serde::Deserialize;
use std::time::Duration;

const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/yash27-lab/primer-scout/releases/latest";
const USER_AGENT: &str = "primer-scout-cli";

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub install_command: String,
}

#[derive(Debug, Deserialize)]
struct ReleasePayload {
    tag_name: String,
}

pub fn check_for_update(current_version: &str) -> Option<UpdateInfo> {
    if std::env::var_os("PRIMER_SCOUT_NO_UPDATE_CHECK").is_some() {
        return None;
    }

    let current = Version::parse(current_version).ok()?;
    let latest_tag = fetch_latest_tag().ok()?;
    let normalized = latest_tag.trim().trim_start_matches('v');
    let latest = Version::parse(normalized).ok()?;

    if latest > current {
        return Some(UpdateInfo {
            latest_version: latest.to_string(),
            install_command:
                "cargo install --git https://github.com/yash27-lab/primer-scout --branch main --force"
                    .to_string(),
        });
    }

    None
}

fn fetch_latest_tag() -> anyhow::Result<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(450))
        .timeout_read(Duration::from_millis(900))
        .timeout_write(Duration::from_millis(900))
        .build();

    let response = agent
        .get(LATEST_RELEASE_URL)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call()?;

    let payload: ReleasePayload = response.into_json()?;
    Ok(payload.tag_name)
}
