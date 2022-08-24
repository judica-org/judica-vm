use super::*;
pub async fn push_to_peer<C: Verification + 'static>(
    secp: Arc<Secp256k1<C>>,
    client: AttestationClient,
    url: (String, u16),
    conn: MsgDB,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    Ok(())
}
