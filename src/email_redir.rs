use core::fmt;
use std::fmt::Display;

use crate::client::OvhClient;
use reqwest::Response;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct OvhMailRedir {
    id: String,
    from: String,
    to: String,
}

impl OvhMailRedir {
    async fn get_redir(
        client: &OvhClient,
        domain: &str,
        id: &str,
    ) -> Result<OvhMailRedir, Box<dyn std::error::Error>> {
        let res = client
            .get(&format!("/email/domain/{}/redirection/{}", domain, id))
            .await?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn list_redirs(
        client: &OvhClient,
        domain: &str,
    ) -> Result<Vec<OvhMailRedir>, Box<dyn std::error::Error>> {
        let resp = client
            .get(&format!("/email/domain/{}/redirection", domain))
            .await?;
        let resp = resp.error_for_status()?;

        let res = resp.json::<Vec<String>>().await?;
        let res: Vec<_> =
            futures::future::join_all(res.iter().map(|id| Self::get_redir(client, domain, id)))
                .await;

        let res = res.into_iter().filter_map(|c| c.ok()).collect();

        Ok(res)
    }

    pub async fn create(
        c: &OvhClient,
        domain: &str,
        from: &str,
        to: &str,
        local_copy: bool,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let data = OvhMailRedirCreate {
            from,
            to,
            local_copy,
        };
        c.post(&format!("/email/domain/{}/redirection", domain), &data)
            .await
    }

    pub async fn delete(
        c: &OvhClient,
        domain: &str,
        id: &str,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        c.delete(&format!("/email/domain/{}/redirection/{}", domain, id))
            .await
    }
}

#[derive(Debug, Serialize)]
struct OvhMailRedirCreate<'a> {
    from: &'a str,
    to: &'a str,

    #[serde(rename(serialize = "localCopy"))]
    local_copy: bool,
}

impl Display for OvhMailRedir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} -> {}", self.id, self.from, self.to)
    }
}
