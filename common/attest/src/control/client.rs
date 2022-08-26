use attest_messages::Envelope;
use reqwest::Client;
use serde_json::Value;

use super::query::{PushMsg, Subscribe};

#[derive(Clone)]
pub struct ControlClient(pub Client);

impl AsRef<Client> for &'_ ControlClient {
    fn as_ref(&self) -> &Client {
        &self.0
    }
}

impl ControlClient {
    pub async fn make_genesis(
        &self,
        nickname: &String,
        url: &String,
        port: u16,
    ) -> Result<Envelope, reqwest::Error> {
        let resp = self
            .as_ref()
            .post(format!("http://{}:{}/make_genesis", url, port))
            .json(nickname)
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }
    pub async fn push_message_dangerous(
        &self,
        p: &PushMsg,
        url: &String,
        port: u16,
    ) -> Result<Value, reqwest::Error> {
        let resp = self
            .as_ref()
            .post(format!("http://{}:{}/push_message_dangerous", url, port))
            .json(p)
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }
    pub async fn add_service(
        &self,
        sub: &Subscribe,
        url: &String,
        port: u16,
    ) -> Result<Value, reqwest::Error> {
        let resp = self
            .as_ref()
            .post(format!("http://{}:{}/service", url, port))
            .json(sub)
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }
}
