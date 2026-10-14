use super::allocations::build;
use crate::{crypto, domain::*};
use alloy_primitives::{Address, B256, U256};
use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

fn campaign(kind: &str, pool: &str, mode: &str, inventory: Vec<&str>) -> Campaign {
    let now = Utc::now();
    let creator = Address::repeat_byte(9);
    let token = Address::repeat_byte(8);
    let spec = json!({"title":"Allocation test","description":"","chain_id":"5042","reward":{"asset_kind":kind,"token_address":token,"amount_base_units":pool,"token_id":if kind=="ERC1155"{"42"}else{"0"},"nft_inventory":inventory},"distribution":{"mode":mode,"winner_count":if mode=="RAFFLE"{2}else{0},"capacity":3,"registration_limit":3,"allocation_policy":"EQUAL_POOL","reward_per_recipient":null},"start_at":now,"cutoff_at":now+Duration::hours(1),"review_deadline":now+Duration::hours(2),"claim_deadline":now+Duration::hours(26),"refund_recipient":null,"tasks":[{"task_type":"X_REPOST","target_url":"https://x.com/a/status/1","instructions":"","required":true}]});
    Campaign {
        id: Uuid::new_v4(),
        campaign_key: vec![1; 32],
        creator_id: Uuid::new_v4(),
        creator_wallet: creator.to_vec(),
        chain_id: 5042,
        escrow_address: Some(Address::repeat_byte(7).to_vec()),
        title: "Allocation test".into(),
        description: "".into(),
        status: "ELIGIBILITY_LOCKED".into(),
        mode: mode.into(),
        start_at: now,
        cutoff_at: now + Duration::hours(1),
        review_deadline: now + Duration::hours(2),
        claim_deadline: now + Duration::hours(26),
        capacity: 3,
        registration_limit: 3,
        registered_count: 3,
        winner_count: if mode == "RAFFLE" { 2 } else { 0 },
        spec_json: spec,
        rules_json: None,
        rules_hash: Some(vec![2; 32]),
        config_hash: Some(vec![3; 32]),
        activated: true,
        chain_state: 1,
        version: 0,
        listed: true,
        moderation_hidden: false,
        moderation_version: 0,
        moderation_reason: None,
        moderation_updated_at: None,
        created_at: now,
    }
}
fn entries(n: usize) -> Vec<(Uuid, Address)> {
    (1..=n)
        .map(|i| (Uuid::new_v4(), Address::repeat_byte(i as u8)))
        .collect()
}
#[test]
fn equal_pool_rounding_and_empty() {
    let c = campaign("ERC20", "10", "ALL_ELIGIBLE", vec![]);
    let r = build(&c, B256::ZERO, &entries(3), None).unwrap();
    assert_eq!(r.total, U256::from(9));
    assert!(r.allocations.iter().all(|a| a.quantity == "3"));
    let r = build(&c, B256::ZERO, &[], None).unwrap();
    assert_eq!(r.root, B256::ZERO);
    assert_eq!(r.total, U256::ZERO);
    assert!(r.allocations.is_empty());
}
#[test]
fn positive_minimum_and_u256() {
    let c = campaign("ERC20", "1", "ALL_ELIGIBLE", vec![]);
    assert!(build(&c, B256::ZERO, &entries(3), None).is_err());
    assert!(amount("01").is_err());
    assert!(amount("-1").is_err());
    assert!(amount("1e6").is_err());
    assert!(
        amount("115792089237316195423570985008687907853269984665640564039457584007913129639936")
            .is_err()
    );
    assert_eq!(amount(&U256::MAX.to_string()).unwrap(), U256::MAX);
}
#[test]
fn fixed_reward_and_erc1155_id() {
    let mut c = campaign("ERC1155", "30", "ALL_ELIGIBLE", vec![]);
    c.spec_json["distribution"]["allocation_policy"] = json!("FIXED_REWARD");
    c.spec_json["distribution"]["reward_per_recipient"] = json!("10");
    let r = build(&c, B256::ZERO, &entries(2), None).unwrap();
    assert_eq!(r.total, U256::from(20));
    assert!(
        r.allocations
            .iter()
            .all(|a| a.token_id == "42" && a.quantity == "10")
    );
}
#[test]
fn nft_assignment_is_unique_and_numeric() {
    let c = campaign("ERC721", "3", "ALL_ELIGIBLE", vec!["100", "0", "5"]);
    let r = build(&c, B256::ZERO, &entries(3), None).unwrap();
    assert_eq!(
        r.allocations
            .iter()
            .map(|a| a.token_id.as_str())
            .collect::<Vec<_>>(),
        vec!["0", "5", "100"]
    );
    assert!(r.allocations.iter().all(|a| a.quantity == "1"));
    assert_eq!(r.total, U256::from(3));
}
#[test]
fn raffle_replay_and_domain_binding() {
    let c = campaign("ERC721", "3", "RAFFLE", vec!["100", "0", "5"]);
    let e = entries(3);
    let seed = B256::repeat_byte(6);
    let a = build(&c, B256::repeat_byte(4), &e, Some(seed)).unwrap();
    let b = build(&c, B256::repeat_byte(4), &e, Some(seed)).unwrap();
    assert_eq!(a.root, b.root);
    assert_eq!(a.transcript, b.transcript);
    assert_eq!(a.allocations.len(), 2);
    let mut changed = c.clone();
    changed.escrow_address = Some(Address::repeat_byte(20).to_vec());
    let foreign = build(&changed, B256::repeat_byte(4), &e, Some(seed)).unwrap();
    assert_ne!(foreign.root, a.root);
    assert!(!crypto::verify_proof(
        a.root,
        foreign.leaves[0],
        &foreign.allocations[0].proof
    ));
}
#[test]
fn immutable_deadline_validation() {
    let c = campaign("ERC20", "10", "ALL_ELIGIBLE", vec![]);
    let mut s = c.spec().unwrap();
    s.start_at = chrono::DateTime::from_timestamp(Utc::now().timestamp() + 60, 0).unwrap();
    s.cutoff_at = s.start_at + Duration::hours(1);
    s.review_deadline = s.cutoff_at + Duration::hours(1);
    s.claim_deadline = s.review_deadline + Duration::hours(24);
    assert!(s.validate(5042, Utc::now()).is_ok());
    s.cutoff_at = s.start_at;
    assert!(s.validate(5042, Utc::now()).is_err());
}

#[test]
fn duplicate_and_unordered_snapshot_is_rejected() {
    let c = campaign("ERC20", "10", "ALL_ELIGIBLE", vec![]);
    let mut e = entries(3);
    e[1].1 = e[0].1;
    assert!(build(&c, B256::ZERO, &e, None).is_err());
    let mut e = entries(3);
    e.reverse();
    assert!(build(&c, B256::ZERO, &e, None).is_err());
}

#[test]
#[ignore = "Local performance baseline; run with --release and --ignored"]
fn large_distribution_performance() {
    let mut c = campaign("ERC20", "100000000000", "ALL_ELIGIBLE", vec![]);
    c.spec_json["distribution"]["registration_limit"] = json!(100_000);
    let entries: Vec<_> = (1..=10_000u64)
        .map(|i| {
            let bytes = U256::from(i).to_be_bytes::<32>();
            (Uuid::new_v4(), Address::from_slice(&bytes[12..]))
        })
        .collect();
    let started = std::time::Instant::now();
    let r = build(&c, B256::ZERO, &entries, None).unwrap();
    let build_ms = started.elapsed().as_secs_f64() * 1_000.0;
    assert_eq!(r.allocations.len(), 10_000);
    let bytes = crypto::canonical(&json!(r.allocations)).unwrap();
    let report = json!({"profile":if cfg!(debug_assertions){"debug"}else{"release"},"recipients":10_000,"build_and_verify_ms":build_ms,"canonical_manifest_bytes":bytes.len(),"build_and_serialize_ms":started.elapsed().as_secs_f64()*1_000.0});
    std::fs::create_dir_all("target").unwrap();
    std::fs::write(
        "target/allocation-performance.json",
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("ALLOCATION_PERFORMANCE {report}");
}
