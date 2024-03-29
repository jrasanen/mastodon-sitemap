use chrono::prelude::*;
use megalodon::{entities, Megalodon};
use megalodon::entities::Account;
use megalodon::error::Error;
use sitewriter::{ChangeFreq, Url, UrlEntry};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

static USER_AGENT: &str = "mastodon-sitemap/0.0.1";

fn get_from_env(key: &str, example: &str) -> String {
    let data = std::env::var(key).expect(&format!("{} must be set. For example: {}", key, example));

    if data.is_empty() {
        panic!("{} must not be empty", key);
    }

    data
}

fn get_from_env_opt(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or(default.parse().unwrap())
}

/// Fetch tags from public timeline
async fn fetch_tags(client: &Box<dyn Megalodon + Send + Sync>) -> Result<Vec<UrlEntry>, Error> {
    let mut sitemap_urls = vec![];

    let get_public_timeline_opts = megalodon::megalodon::GetTimelineOptions {
        only_media: Some(false),
        limit: Some(100),
        max_id: None,
        since_id: None,
        min_id: None,
    };

    let public_timeline = client
        .get_public_timeline(Some(&get_public_timeline_opts))
        .await
        .map_or_else(
            |e| {
                println!("Error: {:?}", e);
                vec![]
            },
            |statuses| statuses.json(),
        );

    let mut tags = vec![];

    for status in public_timeline {
        for tag in &status.tags {
            if !tags.contains(&tag.name) {
                tags.push(tag.url.clone());
            }
        }
    }

    for tag in tags {
        sitemap_urls.push(UrlEntry {
            loc: tag.parse().unwrap(),
            changefreq: Some(ChangeFreq::Daily),
            priority: Some(0.3),
            lastmod: Some(Utc::now()),
        });
    }

    Ok(sitemap_urls)
}

/// Get statuses for account
async fn get_statuses(
    client: &Box<dyn Megalodon + Send + Sync>,
    account_id: String,
) -> Result<Vec<UrlEntry>, Error> {
    let mut sitemap_urls = vec![];
    let account_statuses = client
        .get_account_statuses(account_id, None)
        .await
        .map_or_else(|e| vec![], |statuses| statuses.json());

    for status in account_statuses {
        if status.visibility == entities::StatusVisibility::Public && status.url.is_some() {
            let last_modified = status.edited_at.unwrap_or(status.created_at);
            sitemap_urls.push(UrlEntry {
                loc: status.url.unwrap().parse().unwrap(),
                changefreq: Some(ChangeFreq::Yearly),
                priority: Some(1.0),
                lastmod: Some(last_modified),
            })
        }
    }

    Ok(sitemap_urls)
}

/// Find account by username
async fn find_account(
    client: &Box<dyn Megalodon + Send + Sync>,
    account_username: String,
) -> Result<Account, Error> {
    let results = client
        .search_account(account_username.clone(), None)
        .await
        .map_or_else(|e| vec![], |accounts| accounts.json());

    let account = results
        .iter()
        .find_map(|account| {
            if account.username == account_username {
                Some(account.clone())
            } else {
                None
            }
        })
        .expect("Account not found");

    Ok(account)
}

#[tokio::main]
async fn main() {
    let instance_url = get_from_env("INSTANCE_URL", "https://mastodon.social");
    let access_token = get_from_env("ACCESS_TOKEN", "foobar1234");
    let account_username = get_from_env("ACCOUNT_USERNAME", "YourUsername");
    let output_directory = get_from_env_opt("OUTPUT_DIRECTORY", "");

    let client = megalodon::generator(
        megalodon::SNS::Mastodon,
        instance_url.clone(),
        Some(access_token.clone()),
        Option::from(USER_AGENT.to_string()),
    );

    let mut sitemap_urls = vec![];

    let account = find_account(&client, account_username).await.unwrap();
    let account_id = account.id.clone();
    let account_url = account.url.clone();

    sitemap_urls.push(UrlEntry {
        loc: Url::from_str(&instance_url).unwrap(),
        changefreq: Some(ChangeFreq::Hourly),
        priority: Some(1.0),
        lastmod: Some(Utc::now()),
    });

    sitemap_urls.push(UrlEntry {
        loc: account_url.parse().unwrap(),
        changefreq: Some(ChangeFreq::Daily),
        priority: Some(1.0),
        lastmod: Some(Utc::now()),
    });

    let account_statuses = get_statuses(&client, account_id).await.unwrap_or_default();
    sitemap_urls.extend(account_statuses);

    let tags = fetch_tags(&client).await.unwrap_or_default();
    sitemap_urls.extend(tags);

    let sitemap = sitewriter::generate_str(&sitemap_urls);

    let output_directory = PathBuf::from(output_directory);
    let file_path = output_directory.join("sitemap.xml");

    std::fs::remove_file(file_path.clone()).unwrap_or_default();

    if let Ok(mut file) = File::create(file_path.as_os_str()) {
        file.write_all(sitemap.as_bytes()).unwrap();
    }

    println!("{} written", file_path.to_str().expect("Getting filepath went wrong"));
}
