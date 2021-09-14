use configparser::ini::Ini;
use reqwest::{header::HeaderMap, Response};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub fn new(
        endpoint: &str,
        application_key: &str,
        application_secret: &str,
        consumer_key: &str,
    ) -> Option<OvhClient> {
        let client = reqwest::Client::new();

        let endpoint = ENDPOINTS.get(endpoint)?;
        let application_key = application_key.into();
        let application_secret = application_secret.into();
        let consumer_key = consumer_key.into();

        Some(OvhClient {
            endpoint,
            application_key,
            application_secret,
            consumer_key,
            client,
        })
    }

    pub fn from_conf(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
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

        let c = Self::new(
            &endpoint,
            &application_key,
            &application_secret,
            &consumer_key,
        )
        .ok_or("failed to create client")?;

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

    pub async fn time_delta(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let server_time: u64 = self.get_noauth("/auth/time").await?.text().await?.parse()?;
        Ok(now() - server_time)
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
    ) -> Result<HeaderMap, Box<dyn std::error::Error>> {
        let mut headers = self.default_headers();

        let time_delta = self.time_delta().await?;
        let timestamp = now() + time_delta;
        let timestamp = timestamp.to_string();

        let signature = self.signature(url, &timestamp, method, body);

        insert_sensitive_header(&mut headers, "X-Ovh-Consumer", &self.consumer_key);
        insert_sensitive_header(&mut headers, "X-Ovh-Timestamp", &timestamp);
        insert_sensitive_header(&mut headers, "X-Ovh-Signature", &signature);

        Ok(headers)
    }

    pub async fn get(&self, path: &str) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
        let url = self.url(path);
        let headers = self.gen_headers(&url, "GET", "").await?;

        let resp = self.client.get(url).headers(headers).send().await?;
        Ok(resp)
    }

    pub async fn delete(
        &self,
        path: &str,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
        let url = self.url(path);
        let headers = self.gen_headers(&url, "DELETE", "").await?;

        let resp = self.client.delete(url).headers(headers).send().await?;
        Ok(resp)
    }

    pub async fn post<T: Serialize + ?Sized>(
        &self,
        path: &str,
        data: &T,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let url = self.url(path);

        // Cannot call RequestBuilder.json directly because of body
        // signature requirement.
        let body = serde_json::to_string(data)?;
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

    pub async fn get_noauth(
        &self,
        path: &str,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
        let url = self.url(path);
        let headers = self.default_headers();

        let resp = self.client.get(url).headers(headers).send().await?;
        Ok(resp)
    }
}
