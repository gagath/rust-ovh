//! Low-level access to the OVH API.

use configparser::ini::Ini;
use reqwest::{header::HeaderMap, Response};
use serde::Serialize;
use std::{convert::TryInto, path::Path, result, time::{SystemTime, UNIX_EPOCH}};

// Private data

static ENDPOINTS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "ovh-eu" => "https://eu.api.ovh.com/1.0",
    "ovh-us" => "https://api.us.ovhcloud.com/1.0",
    "ovh-ca" => "https://ca.api.ovh.com/1.0",
    "kimsufi-eu" => "https://eu.api.kimsufi.com/1.0",
    "kimsufi-ca" => "https://ca.api.kimsufi.com/1.0",
    "soyoustart-eu" => "https://eu.api.soyoustart.com/1.0",
    "soyoustart-ca" => "https://ca.api.soyoustart.com/1.0",
};

// Private helpers

fn insert_sensitive_header(
    headers: &mut reqwest::header::HeaderMap,
    header_name: &'static str,
    value: &str,
) {
    let mut header_value = reqwest::header::HeaderValue::from_str(value).unwrap();
    header_value.set_sensitive(true);
    headers.insert(header_name, header_value);
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// Public API

pub type Result<T> = result::Result<T, Box<dyn std::error::Error>>;

pub struct OvhClient {
    endpoint: &'static str,
    application_key: String,
    application_secret: String,
    consumer_key: String,
    client: reqwest::Client,
    time_delta: i64,
}

impl OvhClient {
    /// Creates a new client from scratch.
    ///
    /// This function will perform an API call to get the server time.
    ///
    /// ```no_run
    /// use ovh::client::OvhClient;
    ///
    /// let app_key = "my_app_key";
    /// let app_secret = "my_app_secret";
    /// let consumer_key = "my_consumer_key";
    ///
    /// let client = OvhClient::new("ovh-eu", app_key, app_secret, consumer_key).await.unwrap();
    /// ```
    pub async fn new(
        endpoint: &str,
        application_key: &str,
        application_secret: &str,
        consumer_key: &str,
    ) -> Result<OvhClient> {
        let endpoint = ENDPOINTS.get(endpoint).ok_or("unknown endpoint")?;
        let application_key = application_key.into();
        let application_secret = application_secret.into();
        let consumer_key = consumer_key.into();

        let client = reqwest::Client::new();

        let mut ovh_client = OvhClient {
            endpoint,
            application_key,
            application_secret,
            consumer_key,
            client,
            time_delta: 0,
        };

        let server_time: i64 = ovh_client.get_noauth("/auth/time").await?.text().await?.parse()?;
        let now: i64 = now().try_into()?;
        ovh_client.time_delta = now - server_time;

        Ok(ovh_client)
    }

    /// Creates a new client from a configuration file.
    ///
    /// The configuration file format is usually named `ovh.conf` and
    /// is the same format as the one used in the
    /// [python-ovh](https://github.com/ovh/python-ovh) library:
    ///
    /// ```ini
    /// [default]
    /// ; general configuration: default endpoint
    /// endpoint=ovh-eu
    ///
    /// [ovh-eu]
    /// ; configuration specific to 'ovh-eu' endpoint
    /// application_key=my_app_key
    /// application_secret=my_application_secret
    /// ; uncomment following line when writing a script application
    /// ; with a single consumer key.
    /// ;consumer_key=my_consumer_key
    /// ```
    ///
    /// This function will perform an API call to get the server time.
    pub async fn from_conf<T>(path: T) -> Result<Self>
    where
        T: AsRef<Path>,
    {
        let mut conf = Ini::new();
        conf.load(path)?;

        let endpoint = conf
            .get("default", "endpoint")
            .ok_or("missing key `endpoint`")?;
        let application_key = conf
            .get(&endpoint, "application_key")
            .ok_or("missing key `application_key`")?;
        let application_secret = conf
            .get(&endpoint, "application_secret")
            .ok_or("missing key `application_secret`")?;
        let consumer_key = conf
            .get(&endpoint, "consumer_key")
            .ok_or("missing key `consumer_key`")?;

        Self::new(
            &endpoint,
            &application_key,
            &application_secret,
            &consumer_key,
        ).await
    }

    fn signature(&self, url: &str, timestamp: &str, method: &str, body: &str) -> String {
        let values = [
            &self.application_secret,
            &self.consumer_key,
            method,
            url,
            body,
            timestamp,
        ];
        let sha = sha1::Sha1::from(values.join("+")).hexdigest();
        format!("$1${}", sha)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", &self.endpoint, path)
    }

    /// Retrieves the time delta in seconds between the local machine and the API server
    /// (machine_unix_epoch - server_unix_epoch).
    ///
    /// The delta was determined by performing a request to the API server to get its
    /// time, and then subtract it from the local machine time.
    /// The result is a time delta value, is seconds, that shouldn't be far from 0 if the local
    /// clock is correctly synchronized.
    pub fn time_delta(&self) -> i64 {
        self.time_delta
    }

    fn default_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "X-Ovh-Application",
            reqwest::header::HeaderValue::from_str(&self.application_key).unwrap(),
        );
        headers
    }

    async fn gen_headers(&self, url: &str, method: &str, body: &str) -> Result<HeaderMap> {
        let mut headers = self.default_headers();

        if !body.is_empty() {
            headers.insert(
                "Content-Type",
                reqwest::header::HeaderValue::from_str("application/json; charset=utf-8").unwrap(),
            );
        }

        let now: i64 = now().try_into()?;
        let timestamp = now + self.time_delta;
        let timestamp = timestamp.to_string();

        let signature = self.signature(url, &timestamp, method, body);

        insert_sensitive_header(&mut headers, "X-Ovh-Consumer", &self.consumer_key);
        insert_sensitive_header(&mut headers, "X-Ovh-Timestamp", &timestamp);
        insert_sensitive_header(&mut headers, "X-Ovh-Signature", &signature);

        Ok(headers)
    }

    /// Performs a GET request.
    pub async fn get(&self, path: &str) -> Result<reqwest::Response> {
        let url = self.url(path);
        let headers = self.gen_headers(&url, "GET", "").await?;

        let resp = self.client.get(url).headers(headers).send().await?;
        Ok(resp)
    }

    /// Performs a DELETE request.
    pub async fn delete(
        &self,
        path: &str,
    ) -> Result<reqwest::Response> {
        let url = self.url(path);
        let headers = self.gen_headers(&url, "DELETE", "").await?;

        let resp = self.client.delete(url).headers(headers).send().await?;
        Ok(resp)
    }

    /// Performs a POST request.
    pub async fn post<T: Serialize + ?Sized>(&self, path: &str, data: &T) -> Result<Response> {
        // Cannot call RequestBuilder.json directly because of body
        // signature requirement.
        let body = serde_json::to_string(data)?;
        self.post_raw(path, body).await
    }

    /// Performs a POST request with an empty body.
    pub async fn post_empty(&self, path: &str) -> Result<Response> {
        self.post_raw(path, String::from("")).await
    }

    async fn post_raw(&self, path: &str, body: String) -> Result<Response> {
        let url = self.url(path);
        let headers = self.gen_headers(&url, "POST", &body).await?;

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .body(body)
            .send()
            .await?;

        Ok(resp)
    }

    /// Performs a GET request without auth.
    pub async fn get_noauth(&self, path: &str) -> Result<reqwest::Response> {
        let url = self.url(path);
        let headers = self.default_headers();

        let resp = self.client.get(url).headers(headers).send().await?;
        Ok(resp)
    }
}
