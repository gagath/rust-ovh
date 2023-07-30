//! Low-level access to the OVH API.

use configparser::ini::Ini;
use reqwest::{header::HeaderMap, Response};
use serde::Serialize;
use std::{
    convert::TryInto,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use crate::error::OvhError;

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

pub struct OvhClient {
    endpoint: &'static str,
    application_key: String,
    application_secret: String,
    consumer_key: String,
    client: reqwest::Client,
}

impl OvhClient {
    /// Creates a new client from scratch.
    ///
    /// ```
    /// use ovh::client::OvhClient;
    ///
    /// let app_key = "my_app_key";
    /// let app_secret = "my_app_secret";
    /// let consumer_key = "my_consumer_key";
    ///
    /// let client = OvhClient::new("ovh-eu", app_key, app_secret, consumer_key);
    /// assert!(client.is_some());
    ///
    /// let client = OvhClient::new("wrong-endpoint", app_key, app_secret, consumer_key);
    /// assert!(client.is_none());
    /// ```
    pub fn new(
        endpoint: &str,
        application_key: &str,
        application_secret: &str,
        consumer_key: &str,
    ) -> Option<OvhClient> {
        let endpoint = ENDPOINTS.get(endpoint)?;
        let application_key = application_key.into();
        let application_secret = application_secret.into();
        let consumer_key = consumer_key.into();

        let client = reqwest::Client::new();

        Some(OvhClient {
            endpoint,
            application_key,
            application_secret,
            consumer_key,
            client,
        })
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
    pub fn from_conf<T>(path: T) -> Result<Self, OvhError>
    where
        T: AsRef<Path>,
    {
        let mut conf = Ini::new();
        conf.load(path).map_err(|e| OvhError::Generic(e))?;

        let endpoint = conf
            .get("default", "endpoint")
            .ok_or(OvhError::Generic("missing key `endpoint`".to_owned()))?;
        let application_key = conf
            .get(&endpoint, "application_key")
            .ok_or(OvhError::Generic("missing key `application_key`".to_owned()))?;
        let application_secret = conf
            .get(&endpoint, "application_secret")
            .ok_or(OvhError::Generic("missing key `application_secret`".to_owned()))?;
        let consumer_key = conf
            .get(&endpoint, "consumer_key")
            .ok_or(OvhError::Generic("missing key `consumer_key`".to_owned()))?;

        let c = Self::new(
            &endpoint,
            &application_key,
            &application_secret,
            &consumer_key,
        )
        .ok_or(OvhError::Generic("failed to create client".to_owned()))?;

        Ok(c)
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

    /// Retrieves the time delta between the local machine and the API server.
    ///
    /// This method will perform a request to the API server to get its
    /// local time, and then subtract it from the local time of the machine.
    /// The result is a time delta value, is seconds.
    pub async fn time_delta(&self) -> Result<i64, OvhError> {
        let server_time: u64 = self.get_noauth("/auth/time").await?.text().await.map_err(|e| OvhError::Reqwest)?.parse().map_err(|e| OvhError::ParseIntError)?;

        let delta = (now() - server_time).try_into().map_err(|e| OvhError::TryFromInt)?;
        Ok(delta)
    }

    fn default_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "X-Ovh-Application",
            reqwest::header::HeaderValue::from_str(&self.application_key).unwrap(),
        );
        headers
    }

    async fn gen_headers(
        &self,
        url: &str,
        method: &str,
        body: &str,
    ) -> Result<HeaderMap, OvhError> {
        let mut headers = self.default_headers();

        let time_delta = self.time_delta().await?;
        let now: i64 = now().try_into().map_err(|e| OvhError::TryFromInt)?;
        let timestamp = now + time_delta;
        let timestamp = timestamp.to_string();

        let signature = self.signature(url, &timestamp, method, body);

        insert_sensitive_header(&mut headers, "X-Ovh-Consumer", &self.consumer_key);
        insert_sensitive_header(&mut headers, "X-Ovh-Timestamp", &timestamp);
        insert_sensitive_header(&mut headers, "X-Ovh-Signature", &signature);

        if !body.is_empty() {
            headers.insert("Content-Type", "application/json; charset=utf-8".parse().unwrap());
        }

        Ok(headers)
    }

    /// Performs a GET request.
    pub async fn get(&self, path: &str) -> Result<reqwest::Response, OvhError> {
        let url = self.url(path);
        let headers = self.gen_headers(&url, "GET", "").await?;

        let resp = self.client.get(url).headers(headers).send().await.map_err(|e| OvhError::Reqwest)?;
        Ok(resp)
    }

    /// Performs a DELETE request.
    pub async fn delete(
        &self,
        path: &str,
    ) -> Result<reqwest::Response, OvhError> {
        let url = self.url(path);
        let headers = self.gen_headers(&url, "DELETE", "").await?;

        let resp = self.client.delete(url).headers(headers).send().await.map_err(|e| OvhError::Reqwest)?;
        Ok(resp)
    }

    /// Performs a POST request.
    pub async fn post<T: Serialize + ?Sized>(
        &self,
        path: &str,
        data: &T,
    ) -> Result<Response, OvhError> {
        let url = self.url(path);

        // Cannot call RequestBuilder.json directly because of body
        // signature requirement.
        let body = serde_json::to_string(data).map_err(|e| OvhError::Serde)?;
        let headers = self.gen_headers(&url, "POST", &body).await?;

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .body(body)
            .send()
            .await.map_err(|e| OvhError::Reqwest)?;
        Ok(resp)
    }

    /// Performs a PUT request.
    pub async fn put<T: Serialize + ?Sized>(
        &self,
        path: &str,
        data: &T,
    ) -> Result<Response, OvhError> {
        let url = self.url(path);

        // Cannot call RequestBuilder.json directly because of body
        // signature requirement.
        let body = serde_json::to_string(data).map_err(|e| OvhError::Serde)?;
        let headers = self.gen_headers(&url, "PUT", &body).await?;

        let resp = self
            .client
            .put(url)
            .headers(headers)
            .body(body)
            .send()
            .await.map_err(|e| OvhError::Reqwest)?;
        Ok(resp)
    }

    /// Performs a GET request without auth.
    pub async fn get_noauth(
        &self,
        path: &str,
    ) -> Result<reqwest::Response, OvhError> {
        let url = self.url(path);
        let headers = self.default_headers();

        let resp = self.client.get(url).headers(headers).send().await.map_err(|e| OvhError::Reqwest)?;
        Ok(resp)
    }
}
