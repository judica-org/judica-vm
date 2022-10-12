// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::AttestationClient;

impl AttestationClient {
    pub async fn authenticate(
        &self,
        secret: &[u8; 32],
        url: &String,
        port: u16,
    ) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("http://{}:{}/authenticate", url, port))
            .json(secret)
            .send()
            .await?
            .json()
            .await
    }
}
