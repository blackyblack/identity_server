use identity_server::verify::nonce::NonceManager;
use identity_server::verify::nonce_db::DatabaseNonceManager;
use identity_server::verify::random_keypair;

#[async_std::test]
async fn test_database_nonce_manager() {
    let (_priv, user) = random_keypair();
    let manager = DatabaseNonceManager::new("sqlite::memory:").await.unwrap();

    // initial nonce should be 0
    assert_eq!(manager.nonce(&user).await.unwrap(), 0);
    // next nonce should be 1
    assert_eq!(manager.next_nonce(&user).await.unwrap(), 1);
    // nonce should still be 0 until we use it
    assert_eq!(manager.nonce(&user).await.unwrap(), 0);

    manager.use_nonce(&user, 1).await.unwrap();

    // using same nonce again should fail
    assert!(manager.use_nonce(&user, 1).await.is_err());
    assert_eq!(manager.next_nonce(&user).await.unwrap(), 2);

    assert_eq!(manager.nonce(&user).await.unwrap(), 1);
}
