//! High-level access to the DNS records API.

use core::fmt;
use std::fmt::Display;

use futures::future;
use reqwest::Response;
use serde::{Deserialize, Serialize};

use crate::client::OvhClient;

#[derive(Debug, Serialize, Deserialize)]
pub enum DnsRecordType {
    A,
    AAAA,
    CAA,
    CNAME,
    DKIM,
    DMARC,
    DNAME,
    LOC,
    MX,
    NAPTR,
    NS,
    PTR,
    SPF,
    SRV,
    SSHFP,
    TLSA,
    TXT,
}

/// Structure representing a single DNS record.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OvhDnsRecord {
    /// The internal name of the zone
    pub zone: String,
    /// Resource record Name
    pub field_type: DnsRecordType,
    /// Resource record subdomain
    pub sub_domain: Option<String>,
    /// Resource record target
    pub target: String,
    /// Resource record ttl (positive 32 bit signed integer, see: https://www.rfc-editor.org/rfc/rfc2181#section-8)
    pub ttl: Option<i32>,
}

impl OvhDnsRecord {
    /// Retrieves a DNS record
    async fn get_record(
        client: &OvhClient,
        zone_name: &str,
        id: u64,
    ) -> Result<OvhDnsRecord, Box<dyn std::error::Error>> {
        let mut record: OvhDnsRecord = client
            .get(&format!("/domain/zone/{}/record/{}", zone_name, id))
            .await?
            .json()
            .await?;

        // normalize sub_domain
        record.sub_domain = match record.sub_domain {
            Some(sub_domain) if sub_domain.is_empty() => None,
            x => x,
        };

        Ok(record)
    }

    /// Lists all DNS records
    ///
    /// This method will perform one extra API call per record
    /// in order to get their details.
    ///
    /// ```no_run
    /// use ovh::client::OvhClient;
    /// use ovh::dns_record::OvhDnsRecord;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let c = OvhClient::from_conf("ovh.conf").unwrap();
    ///     let records = OvhDnsRecord::list(&c, "example.com")
    ///         .await
    ///         .unwrap();
    ///
    ///     for r in records {
    ///         println!("{}", r);
    ///     }
    /// }
    /// ```
    pub async fn list(
        client: &OvhClient,
        zone_name: &str,
    ) -> Result<Vec<OvhDnsRecord>, Box<dyn std::error::Error>> {
        let records = client
            .get(&format!("/domain/zone/{}/record", zone_name))
            .await?
            .error_for_status()?
            .json::<Vec<u64>>().await?
            .into_iter()
            .map(|id| Self::get_record(client, zone_name, id));

        future::join_all(records)
            .await
            .into_iter()
            .collect()
    }
}

impl Display for OvhDnsRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let domain = match &self.sub_domain {
            Some(sub_domain) => format!("{}.{}.", sub_domain, self.zone),
            None => format!("{}.", self.zone),
        };
        write!(f, "{} {} {:?} {}", domain, self.ttl.unwrap_or(0), self.field_type, self.target)
    }
}
