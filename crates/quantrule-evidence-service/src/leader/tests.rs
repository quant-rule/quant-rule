use super::*;

#[tokio::test]
async fn a_live_lease_is_not_stolen_and_renewal_keeps_its_term() {
    let lease = MemoryLeaderLease::with_lease_seconds(60);
    let first = lease
        .acquire("replica-a".to_owned())
        .await
        .expect("acquire")
        .expect("leader");
    assert_eq!(first, 1);
    assert_eq!(
        lease
            .acquire("replica-b".to_owned())
            .await
            .expect("second acquire"),
        None
    );
    assert_eq!(
        lease
            .renew("replica-a".to_owned(), first)
            .await
            .expect("renew"),
        Some(first),
        "a renewal must not increment the term"
    );
    assert_eq!(
        lease
            .renew("replica-b".to_owned(), first)
            .await
            .expect("wrong holder"),
        None
    );
}

#[tokio::test]
async fn an_expired_lease_is_taken_over_with_a_new_term() {
    let lease = MemoryLeaderLease::with_lease_seconds(0);
    assert_eq!(
        lease
            .acquire("replica-a".to_owned())
            .await
            .expect("first acquire"),
        Some(1)
    );
    assert_eq!(
        lease
            .acquire("replica-b".to_owned())
            .await
            .expect("takeover"),
        Some(2),
        "a standby takes over after expiry"
    );
    assert_eq!(
        lease
            .renew("replica-a".to_owned(), 1)
            .await
            .expect("stale renew"),
        None
    );
    let status = lease.current().await.expect("current").expect("row");
    assert_eq!(status.holder, "replica-b");
    assert_eq!(status.term, 2);
}

#[tokio::test]
async fn a_release_expires_the_row_immediately() {
    let lease = MemoryLeaderLease::with_lease_seconds(60);
    let term = lease
        .acquire("replica-a".to_owned())
        .await
        .expect("acquire")
        .expect("leader");
    lease
        .release("replica-a".to_owned(), term)
        .await
        .expect("release");
    assert_eq!(
        lease
            .acquire("replica-b".to_owned())
            .await
            .expect("takeover"),
        Some(term + 1)
    );
}

#[tokio::test]
async fn the_memory_row_reports_expiry_against_its_clock() {
    let lease = MemoryLeaderLease::with_lease_seconds(0);
    let term = lease
        .acquire("replica-a".to_owned())
        .await
        .expect("acquire")
        .expect("leader");
    let row = lease.lock_row();
    assert_eq!(row.live_term(SystemTime::now()), None);
    assert_eq!(row.term, term);
}
