//! High-level access to the DNS records API.

use core::fmt;
use std::fmt::Display;

use futures::future;
use serde::{Deserialize, Serialize};

use crate::client::OvhClient;
use crate::client::Result;

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
#[derive(Debug, Deserialize)]
pub struct OvhDnsRecord {
    /// Unique identifier of the DNS record
    ///
    /// example: `1234567`
    pub id: u64,

    /// The internal name of the zone
    ///
    /// example: `example.com`
    pub zone: String,

    /// Resource record name
    ///
    /// example: `A`
    #[serde(rename = "fieldType")]
    pub record_type: DnsRecordType,

    /// Resource record subdomain
    ///
    /// example: `www`
    #[serde(rename = "subDomain")]
    pub subdomain: Option<String>,

    /// Resource record target
    ///
    /// example: `93.184.216.34`
    pub target: String,

    /// Resource record TTL in seconds (positive 32 bit signed integer, see: https://www.rfc-editor.org/rfc/rfc2181#section-8).
    ///
    /// example: 86400
    pub ttl: Option<i32>,
}

#[derive(Serialize)]
struct OvhDnsRecordCreate<'a> {
    #[serde(rename = "fieldType")]
    pub record_type: DnsRecordType,
    #[serde(rename = "subDomain")]
    pub subdomain: Option<&'a str>,
    pub target: &'a str,
    pub ttl: Option<i32>,
}

impl OvhDnsRecord {
    /// Retrieves the fully qualified domain name (subdomain + zone).
    ///
    /// example
    /// ```
    /// use ovh::dns_record::{DnsRecordType, OvhDnsRecord};
    ///
    /// let record = OvhDnsRecord {
    ///     id: 1234567,
    ///     subdomain: Some(String::from("www")),
    ///     zone: String::from("example.com"),
    ///     ttl: Some(86400),
    ///     record_type: DnsRecordType::A,
    ///     target: String::from("93.184.216.34"),
    /// };
    ///
    /// assert_eq!(record.fqn(), "www.example.com.");
    /// ```
    pub fn fqn(&self) -> String {
        match &self.subdomain {
            Some(subdomain) => format!("{}.{}.", subdomain, self.zone),
            None => format!("{}.", self.zone),
        }
    }

    fn normalize(mut self) -> Self {
        if self.subdomain == Some(String::from("")) {
            self.subdomain = None;
        }

        if self.ttl == Some(0) {
            self.ttl = None;
        }

        self
    }

    /// Retrieves a DNS record.
    async fn get_record(client: &OvhClient, zone_name: &str, id: &u64) -> Result<OvhDnsRecord> {
        let record = client
            .get(&format!("/domain/zone/{}/record/{}", zone_name, id))
            .await?
            .error_for_status()?
            .json::<OvhDnsRecord>()
            .await?
            .normalize();

        Ok(record)
    }

    /// Lists all DNS records.
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
    ///     for record in records {
    ///         println!("{}", record);
    ///	    }
    /// }
    /// ```
    pub async fn list(client: &OvhClient, zone: &str) -> Result<Vec<OvhDnsRecord>> {
        Self::list_filtered(client, zone, None, None).await
    }

    /// Lists all DNS records having the provided type (is set) and subdomain (if set).
    ///
    /// This method will perform one extra API call per record
    /// in order to get their details.
    ///
    /// ```no_run
    /// use ovh::client::OvhClient;
    /// use ovh::dns_record::DnsRecordType;
    /// use ovh::dns_record::OvhDnsRecord;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let c = OvhClient::from_conf("ovh.conf").unwrap();
    ///     let records = OvhDnsRecord::list_filtered(&c, "example.com", Some(DnsRecordType::CNAME), Some(String::from("foo")))
    ///         .await
    ///         .unwrap();
    ///
    ///     for record in records {
    ///         println!("{}", record);
    ///	    }
    /// }
    /// ```
    pub async fn list_filtered(client: &OvhClient, zone: &str, record_type: Option<DnsRecordType>, subdomain: Option<String>) -> Result<Vec<OvhDnsRecord>> {
        let mut options = Vec::with_capacity(2);
        if let Some(record_type) = record_type {
            options.push(format!("fieldType={:?}", record_type))
        }
        if let Some(subdomain) = subdomain {
            options.push(format!("subDomain={}", subdomain))
        }

        let url = if options.is_empty() {
            format!("/domain/zone/{}/record", zone)
        } else {
            format!("/domain/zone/{}/record?{}", zone, options.join("&"))
        };

        let ids = client
            .get(&url)
            .await?
            .error_for_status()?
            .json::<Vec<u64>>().await?;

        let records = ids
            .iter()
            .map(|id| Self::get_record(client, zone, id));

        future::join_all(records)
            .await
            .into_iter()
            .collect()
    }

    /// Creates a new DNS record.
    ///
    /// ```no_run
    /// use ovh::client::OvhClient;
    /// use ovh::dns_record::OvhDnsRecord;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let c = OvhClient::from_conf("ovh.conf").unwrap();
    ///     let record = OvhDnsRecord::create(&c, Some("www"), "example.com", DnsRecordType::A, Some(86400), "93.184.216.34")
    ///         .await
    ///         .unwrap();
    ///     println!("{}", record);
    /// }
    /// ```
    pub async fn create(c: &OvhClient, subdomain: Option<&str>, zone: &str, record_type: DnsRecordType, ttl: Option<i32>, target: &str) -> Result<OvhDnsRecord> {
        let payload = OvhDnsRecordCreate { subdomain, record_type, ttl, target };
        let record = c.post(&format!("/domain/zone/{}/record", zone), &payload)
            .await?
            .error_for_status()?
            .json::<OvhDnsRecord>()
            .await?
            .normalize();

        Self::refresh_zone(c, zone).await?;
        Ok(record)
    }

    /// Deletes an existing DNS record.
    ///
    /// ```no_run
    /// use ovh::client::OvhClient;
    /// use ovh::dns_record::OvhDnsRecord;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let c = OvhClient::from_conf("ovh.conf").unwrap();
    ///     OvhDnsRecord::delete(&c, "example.com", 1234567)
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    pub async fn delete(c: &OvhClient, zone: &str, id: u64) -> Result<()> {
        c.delete(&format!("/domain/zone/{}/record/{}", zone, id))
            .await?
            .error_for_status()?;

        Self::refresh_zone(c, zone).await
    }

    async fn refresh_zone(c: &OvhClient, zone: &str) -> Result<()> {
        c.post_empty(&format!("/domain/zone/{}/refresh", zone))
            .await?
            .error_for_status()?;

        Ok(())
    }
}

impl Display for OvhDnsRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[id: {}] {} {} {:?} {}", self.id, self.fqn(), self.ttl.unwrap_or(0), self.record_type, self.target)
    }
}
