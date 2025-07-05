use identity_server::admins::db::DatabaseAdminStorage;
use identity_server::admins::AdminStorage;
use identity_server::identity::storage::VouchStorage;
use identity_server::identity::storage::ProofStorage;
use identity_server::identity::storage::PenaltyStorage;
use identity_server::identity::storage_db::{
    DatabaseVouchStorage, DatabaseProofStorage, DatabasePenaltyStorage,
};
use identity_server::identity::{ModeratorProof, SystemPenalty, UserAddress, IdtAmount};

use std::collections::{HashMap, HashSet};

#[async_std::test]
async fn test_database_admin_storage() {
    let admin = "admin".to_string();
    let moderator = "mod".to_string();
    let others = "user".to_string();
    let admins = HashSet::from([admin.clone()]);
    let moderators = HashSet::from([moderator.clone()]);
    let storage = DatabaseAdminStorage::new("sqlite::memory:", admins, moderators)
        .await
        .unwrap();

    assert!(storage.check_admin(&admin).await.is_ok());
    assert!(storage.check_admin(&others).await.is_err());
    assert!(storage.check_moderator(&moderator).await.is_ok());

    let admin2 = "admin2".to_string();
    storage.add_admin(&admin, admin2.clone()).await.unwrap();
    assert!(storage.check_admin(&admin2).await.is_ok());
    storage.remove_admin(&admin, admin2.clone()).await.unwrap();
    assert!(storage.check_admin(&admin2).await.is_err());

    let mod2 = "mod2".to_string();
    storage.add_moderator(&admin, mod2.clone()).await.unwrap();
    assert!(storage.check_moderator(&mod2).await.is_ok());
    storage.remove_moderator(&admin, mod2.clone()).await.unwrap();
    assert!(storage.check_moderator(&mod2).await.is_err());
}

#[async_std::test]
async fn test_database_vouch_storage() {
    let storage = DatabaseVouchStorage::new("sqlite::memory:").await.unwrap();
    let user_a = "user_a".to_string();
    let user_b = "user_b".to_string();

    storage.vouch(user_a.clone(), user_b.clone(), 1).await.unwrap();
    assert_eq!(
        storage.vouchers_with_time(&user_b).await.unwrap().get(&user_a).copied(),
        Some(1)
    );
    assert_eq!(
        storage.vouchees_with_time(&user_a).await.unwrap().get(&user_b).copied(),
        Some(1)
    );

    storage.vouch(user_a.clone(), user_b.clone(), 5).await.unwrap();
    assert_eq!(
        storage.vouchers_with_time(&user_b).await.unwrap().get(&user_a).copied(),
        Some(5)
    );

    storage.remove_vouch(user_a.clone(), user_b.clone()).await.unwrap();
    assert!(storage.vouchers_with_time(&user_b).await.unwrap().is_empty());
    assert!(storage.vouchees_with_time(&user_a).await.unwrap().is_empty());
}

#[async_std::test]
async fn test_database_proof_storage() {
    let storage = DatabaseProofStorage::new("sqlite::memory:").await.unwrap();
    let user = "user".to_string();
    let moderator = "moderator".to_string();

    let mut genesis = HashMap::<UserAddress, IdtAmount>::new();
    genesis.insert(user.clone(), 100);
    storage.set_genesis(genesis).await.unwrap();
    assert_eq!(storage.genesis_balance(&user).await.unwrap(), Some(100));
    assert_eq!(storage.genesis_balance(&"none".to_string()).await.unwrap(), None);

    let proof1 = ModeratorProof {
        moderator: moderator.clone(),
        amount: 10,
        proof_id: 1,
        timestamp: 1,
    };
    storage.set_proof(user.clone(), proof1.clone()).await.unwrap();
    let res = storage.proof(&user).await.unwrap().unwrap();
    assert_eq!(res.moderator, proof1.moderator);
    assert_eq!(res.amount, proof1.amount);
    assert_eq!(res.proof_id, proof1.proof_id);
    assert_eq!(res.timestamp, proof1.timestamp);

    let proof2 = ModeratorProof {
        moderator: "mod2".to_string(),
        amount: 20,
        proof_id: 2,
        timestamp: 2,
    };
    storage.set_proof(user.clone(), proof2.clone()).await.unwrap();
    let res = storage.proof(&user).await.unwrap().unwrap();
    assert_eq!(res.moderator, proof2.moderator);
    assert_eq!(res.amount, proof2.amount);
    assert_eq!(res.proof_id, proof2.proof_id);
    assert_eq!(res.timestamp, proof2.timestamp);
}

#[async_std::test]
async fn test_database_penalty_storage() {
    let storage = DatabasePenaltyStorage::new("sqlite::memory:").await.unwrap();
    let user = "user".to_string();
    let vouchee = "vouchee".to_string();

    let proof1 = ModeratorProof {
        moderator: "mod".to_string(),
        amount: 1,
        proof_id: 1,
        timestamp: 2,
    };
    storage.insert_moderator_penalty(user.clone(), proof1.clone()).await.unwrap();
    let res = storage.moderator_penalty(&user).await.unwrap().unwrap();
    assert_eq!(res.moderator, proof1.moderator);
    assert_eq!(res.amount, proof1.amount);
    assert_eq!(res.proof_id, proof1.proof_id);
    assert_eq!(res.timestamp, proof1.timestamp);

    let proof2 = ModeratorProof {
        moderator: "mod2".to_string(),
        amount: 3,
        proof_id: 2,
        timestamp: 4,
    };
    storage.insert_moderator_penalty(user.clone(), proof2.clone()).await.unwrap();
    let res = storage.moderator_penalty(&user).await.unwrap().unwrap();
    assert_eq!(res.moderator, proof2.moderator);
    assert_eq!(res.amount, proof2.amount);
    assert_eq!(res.proof_id, proof2.proof_id);
    assert_eq!(res.timestamp, proof2.timestamp);

    let penalty1 = SystemPenalty { amount: 5, timestamp: 6 };
    storage
        .insert_forgotten_penalty(user.clone(), vouchee.clone(), penalty1.clone())
        .await
        .unwrap();
    let res = storage
        .forgotten_penalty(&user, &vouchee)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(res.amount, penalty1.amount);
    assert_eq!(res.timestamp, penalty1.timestamp);
    assert!(storage.forgotten_users(&user).await.unwrap().contains(&vouchee));

    let penalty2 = SystemPenalty { amount: 7, timestamp: 8 };
    storage
        .insert_forgotten_penalty(user.clone(), vouchee.clone(), penalty2.clone())
        .await
        .unwrap();
    let res = storage
        .forgotten_penalty(&user, &vouchee)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(res.amount, penalty2.amount);
    assert_eq!(res.timestamp, penalty2.timestamp);

    storage.remove_forgotten(user.clone(), &vouchee).await.unwrap();
    assert!(storage.forgotten_penalty(&user, &vouchee).await.unwrap().is_none());
}

