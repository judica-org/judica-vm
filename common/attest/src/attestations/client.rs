use attest_messages::Envelope;

use reqwest::Client;

use crate::control::query::Outcome;

use super::query::Tips;

#[derive(Clone)]
pub struct AttestationClient(pub Client);

impl AsRef<Client> for &'_ AttestationClient {
    fn as_ref(&self) -> &Client {
        &self.0
    }
}

impl AttestationClient {
    pub async fn get_latest_tips(
        &self,
        url: &String,
        port: u16,
    ) -> Result<Vec<Envelope>, reqwest::Error> {
        let resp: Vec<Envelope> = self
            .as_ref()
            .get(format!("http://{}:{}/tips", url, port))
            .json(&None::<Tips>)
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }
    pub async fn get_tips(
        &self,
        tips: Tips,
        url: &String,
        port: u16,
    ) -> Result<Vec<Envelope>, reqwest::Error> {
        let resp: Vec<Envelope> = self
            .as_ref()
            .get(format!("http://{}:{}/tips", url, port))
            .json(&tips)
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }

    pub async fn post_messages(
        &self,
        envelopes: &Vec<Envelope>,
        url: &String,
        port: u16,
    ) -> Result<Vec<Outcome>, reqwest::Error> {
        let resp = self
            .as_ref()
            .post(format!("http://{}:{}/msg", url, port))
            .json(envelopes)
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }
}
