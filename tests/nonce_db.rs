use identity_server::verify::nonce::NonceManager;
use identity_server::verify::nonce_db::DatabaseNonceManager;
use identity_server::verify::random_keypair;

#[async_std::test]
async fn test_database_nonce_manager() {
    let (_priv, user) = random_keypair();
    let manager = DatabaseNonceManager::new("sqlite::memory:").await.unwrap();

    // initial nonce should be 0
    assert_eq!(manager.nonce(&user).await.unwrap(), 0);

    let n1 = manager.next_nonce(&user).await.unwrap();
    assert_eq!(n1, 1);
    assert_eq!(manager.nonce(&user).await.unwrap(), 1);

    manager.use_nonce(&user, n1).await.unwrap();

    // using same nonce again should fail
    assert!(manager.use_nonce(&user, n1).await.is_err());

    let n2 = manager.next_nonce(&user).await.unwrap();
    assert_eq!(n2, 2);
}
