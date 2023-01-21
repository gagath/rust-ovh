//! High-level access to the email redirection API.

use core::fmt;
use std::fmt::Display;

use crate::client::OvhClient;
use crate::client::Result;
use reqwest::Response;

use serde::{Deserialize, Serialize};

/// Structure representing a single email redirection.
#[derive(Debug, Deserialize)]
pub struct OvhMailRedir {
    /// Unique identifier of the redirection
    pub id: String,
    /// Email address to redirect from
    pub from: String,
    /// Email address to redirect to
    pub to: String,
}

impl OvhMailRedir {
    /// Retrieves an email redirection entry.
    async fn get_redir(client: &OvhClient, domain: &str, id: &str) -> Result<OvhMailRedir> {
        let res = client
            .get(&format!("/email/domain/{}/redirection/{}", domain, id))
            .await?
            .json()
            .await?;
        Ok(res)
    }

    /// Lists all of the email redirections
    ///
    /// This method will perform one extra API call per redirection
    /// in order to get their details.
    ///
    /// ```no_run
    /// use ovh::client::OvhClient;
    /// use ovh::email_redir::OvhMailRedir;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let c = OvhClient::from_conf("ovh.conf").unwrap();
    ///     let redirs = OvhMailRedir::list(&c, "example.com")
    ///         .await
    ///         .unwrap();
    ///
    ///     for r in redirs {
    ///        println!("{}", r);
    ///     }
    /// }
    /// ```
    pub async fn list(client: &OvhClient, domain: &str) -> Result<Vec<OvhMailRedir>> {
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

    /// Creates a new redirection.
    ///
    /// ```no_run
    /// use ovh::client::OvhClient;
    /// use ovh::email_redir::OvhMailRedir;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let c = OvhClient::from_conf("ovh.conf").unwrap();
    ///     OvhMailRedir::create(&c, "example.com", "foo@example.com", "admin@example.com", false)
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    pub async fn create(c: &OvhClient, domain: &str, from: &str, to: &str, local_copy: bool) -> Result<Response> {
        let data = OvhMailRedirCreate {
            from,
            to,
            local_copy,
        };
        c.post(&format!("/email/domain/{}/redirection", domain), &data)
            .await
    }

    /// Deletes an existing redirection.
    ///
    /// ```no_run
    /// use ovh::client::OvhClient;
    /// use ovh::email_redir::OvhMailRedir;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let c = OvhClient::from_conf("ovh.conf").unwrap();
    ///     OvhMailRedir::delete(&c, "example.com", "1234567")
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    pub async fn delete(c: &OvhClient, domain: &str, id: &str) -> Result<Response> {
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
