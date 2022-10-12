// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use attest_messages::Envelope;
use reqwest::Client;

use super::query::{NewGenesis, Outcome, PushMsg, Subscribe};

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
        new_genesis: &NewGenesis,
        url: &String,
        port: u16,
    ) -> Result<Envelope, reqwest::Error> {
        let resp = self
            .as_ref()
            .post(format!("http://{}:{}/make_genesis", url, port))
            .json(new_genesis)
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
    ) -> Result<Outcome, reqwest::Error> {
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
    ) -> Result<Outcome, reqwest::Error> {
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
