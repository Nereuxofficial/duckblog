use crate::SPONSORS;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sponsor {
    name: String,
    url: Url,
}

#[instrument(err)]
/// Gets the sponsors of the blog from GitHub. Result is cached for 1 hour. So worst case this takes around 300ms, which sucks a bit... Maybe some async task so it is always fast?
pub async fn get_sponsors() -> color_eyre::Result<Vec<Sponsor>> {
    let sponsors = SPONSORS.get().unwrap().read().await.clone();
    Ok(sponsors)
}

#[instrument(err)]
pub async fn noncached_get_sponsors() -> color_eyre::Result<Vec<Sponsor>> {
    let graphql_query = r#"
    {
      viewer {
        sponsorshipsAsMaintainer(first: 100) {
          nodes {
            sponsorEntity {
              ... on User {
                login
                url
              }
            }
          }
        }
      }
    }
    "#;

    // Create a reqwest Client
    let client = Client::new();

    // Send a POST request to the GitHub GraphQL API
    let response = client
        .post("https://api.github.com/graphql")
        .header("User-Agent", "duckblog v1.0")
        .header(
            "Authorization",
            format!("Bearer {}", std::env::var("GITHUB_TOKEN").unwrap()),
        )
        .body(
            serde_json::json!({
                "query": graphql_query,
            })
            .to_string(),
        )
        .send()
        .await?
        .text()
        .await?;

    let response = serde_json::from_str::<serde_json::Value>(&response)?;

    Ok(response
        .get("data")
        .unwrap()
        .get("viewer")
        .unwrap()
        .get("sponsorshipsAsMaintainer")
        .unwrap()
        .get("nodes")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|node| {
            let sponsor = node.get("sponsorEntity").unwrap();
            Sponsor {
                name: sponsor.get("login").unwrap().as_str().unwrap().to_string(),
                url: Url::parse(sponsor.get("url").unwrap().as_str().unwrap()).unwrap(),
            }
        })
        .collect::<Vec<Sponsor>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenvy::dotenv;

    #[tokio::test]
    async fn test_get_sponsors() {
        dotenv().ok();
        let sponsors = noncached_get_sponsors().await.unwrap();
        assert!(!sponsors.is_empty());
    }
}
