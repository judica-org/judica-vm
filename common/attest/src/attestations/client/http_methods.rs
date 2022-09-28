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
