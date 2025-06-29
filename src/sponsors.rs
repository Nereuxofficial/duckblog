use crate::SPONSORS;
use color_eyre::eyre::OptionExt;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sponsor {
    name: String,
    url: Url,
}

#[instrument(err)]
/// Gets the current sponsors of the blog from GitHub.
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
        .ok_or_eyre(format!(
            "Failed to get data from GitHub. Response: {response}",
        ))?
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

// TODO: Get_sponsors test with mocked value
