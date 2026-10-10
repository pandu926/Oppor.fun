use alloy_primitives::{Address, B256, U256, keccak256};
use alloy_sol_types::{SolCall, SolValue, sol};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use chrono::{DateTime, Duration, Utc};
use oppor_backend::{chain::Rpc, config::Config, crypto, domain::Campaign, state::AppState};
use serde_json::{Value, json};
use sqlx::Row;
use std::{
    net::{SocketAddr, TcpListener},
    process::{Child, Command, Stdio},
};
use tower::ServiceExt;
use uuid::Uuid;
use zeroize::Zeroizing;

sol! {
    struct FixtureClaim { uint256 index; address recipient; uint256 tokenId; uint256 quantity; }
    function mint(address to,uint256 amount) external;
    function setPaused(bool value) external;
    function setEnabled(bool value) external;
    function balanceOf(address owner) external view returns(uint256);
    function claimLeaf(FixtureClaim allocation) external view returns(bytes32);
}

struct Node(Child);
impl Drop for Node {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[derive(Clone)]
struct Session {
    cookie: String,
    csrf: String,
    wallet: Address,
    user_id: Uuid,
}
struct Harness {
    state: AppState,
    app: Router,
    _node: Node,
    accounts: Vec<Address>,
    token: Address,
    schema: String,
    admin: sqlx::PgPool,
}
static API_RECORDS: std::sync::Mutex<Vec<Value>> = std::sync::Mutex::new(Vec::new());

async fn response(
    app: &Router,
    method: &str,
    path: &str,
    body: Value,
    session: Option<&Session>,
    key: Option<&str>,
) -> (StatusCode, Value, axum::http::HeaderMap) {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("/v1{path}"))
        .header("origin", "http://localhost:3000")
        .header("content-type", "application/json");
    if let Some(s) = session {
        request = request
            .header("cookie", &s.cookie)
            .header("x-csrf-token", &s.csrf);
    }
    if let Some(k) = key {
        request = request.header("idempotency-key", k);
    }
    let mut req = request
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let ip = session.map(|s| s.wallet.as_slice()[19]).unwrap_or(1);
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 1, ip], 40000))));
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = to_bytes(res.into_body(), 16 * 1024 * 1024).await.unwrap();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"body":String::from_utf8_lossy(&bytes)}));
    API_RECORDS.lock().unwrap().push(
        json!({"method":method.to_lowercase(),"path":path,"status":status.as_u16(),"body":value}),
    );
    (status, value, headers)
}
async fn ok(h: &Harness, method: &str, path: &str, body: Value, s: Option<&Session>) -> Value {
    let key = Uuid::new_v4().to_string();
    let (status, value, _) = response(&h.app, method, path, body, s, Some(&key)).await;
    assert_eq!(status, StatusCode::OK, "{method} {path}: {value}");
    value
}
fn artifact(name: &str) -> Value {
    let path = if matches!(name, "CampaignFactory" | "CampaignEscrow") {
        format!("../contracts/out/{name}.sol/{name}.json")
    } else {
        format!("tests/fixtures/out/BackendFixture.sol/{name}.json")
    };
    serde_json::from_slice(
        &std::fs::read(path).expect("Run bash scripts/test.sh to build contracts"),
    )
    .unwrap()
}
async fn rpc_send(rpc: &Rpc, from: Address, to: Option<Address>, data: String) -> Value {
    let mut tx = json!({"from":from,"data":data,"gas":"0x989680"});
    if let Some(to) = to {
        tx["to"] = json!(to);
    }
    let hash = rpc
        .request("eth_sendTransaction", json!([tx]))
        .await
        .unwrap();
    for _ in 0..100 {
        let receipt = rpc
            .request("eth_getTransactionReceipt", json!([hash]))
            .await
            .unwrap();
        if !receipt.is_null() {
            assert_eq!(
                receipt["status"], "0x1",
                "Failed fixture transaction: {receipt}"
            );
            return receipt;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("Timed out waiting for fixture receipt");
}
async fn deploy(rpc: &Rpc, from: Address, name: &str, args: Vec<u8>) -> Address {
    let code = artifact(name)["bytecode"]["object"]
        .as_str()
        .unwrap()
        .to_owned();
    let receipt = rpc_send(rpc, from, None, format!("{}{}", code, hex::encode(args))).await;
    serde_json::from_value(receipt["contractAddress"].clone()).unwrap()
}
async fn harness() -> Harness {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let node = Node(
        Command::new("anvil")
            .args([
                "--silent",
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--chain-id",
                "5042",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Anvil must be installed"),
    );
    let db_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://oppor:oppor-local-tests@127.0.0.1:55491/oppor".into());
    let admin = sqlx::PgPool::connect(&db_url)
        .await
        .expect("Start compose.test.yml dependencies");
    let schema = format!("test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .unwrap();
    let mut url = url::Url::parse(&db_url).unwrap();
    url.query_pairs_mut()
        .append_pair("options", &format!("-csearch_path={schema}"));
    let config = Config {
        production: false,
        bind: "127.0.0.1:0".parse().unwrap(),
        database_url: url.to_string(),
        redis_url: std::env::var("TEST_REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:56389/0".into()),
        public_origin: "http://localhost:3000".into(),
        origins: vec!["http://localhost:3000".into()],
        trusted_proxies: vec![],
        admin_wallets: vec![],
        domain: "localhost:3000".into(),
        chain_id: 5042,
        rpc_url: format!("http://127.0.0.1:{port}"),
        factory: Address::repeat_byte(1),
        factory_code_hash: B256::repeat_byte(1),
        escrow_code_hash: keccak256(
            hex::decode(
                artifact("CampaignEscrow")["deployedBytecode"]["object"]
                    .as_str()
                    .unwrap()
                    .trim_start_matches("0x"),
            )
            .unwrap(),
        ),
        deployment_block: 0,
        confirmations: 0,
        session_ttl: 3600,
        seed_key: Zeroizing::new(crypto::random32()),
        rate_key: Zeroizing::new(crypto::random32()),
        db_max_connections: 16,
        http_timeout: std::time::Duration::from_secs(15),
        max_indexer_lag: 120,
        storage_endpoint: std::env::var("TEST_STORAGE_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:59009".into()),
        storage_bucket: format!("oppor-{}", Uuid::new_v4().simple()),
        storage_region: "us-east-1".into(),
        storage_access_key: "oppor-local".into(),
        storage_secret_key: Zeroizing::new("oppor-local-storage-tests".into()),
    };
    let rpc = Rpc::new(&config).unwrap();
    let mut accounts = None;
    for _ in 0..100 {
        if let Ok(result) = rpc.request("eth_accounts", json!([])).await {
            accounts = Some(serde_json::from_value::<Vec<Address>>(result).unwrap());
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let accounts = accounts.expect("Anvil did not start");
    let factory = deploy(&rpc, accounts[0], "CampaignFactory", vec![]).await;
    let token = deploy(&rpc, accounts[0], "FixtureToken", vec![]).await;
    let mut config = config;
    config.admin_wallets = vec![accounts[9]];
    config.factory = factory;
    config.factory_code_hash = keccak256(rpc.code(factory).await.unwrap());
    reqwest::Client::new()
        .put(format!(
            "{}/{}",
            config.storage_endpoint, config.storage_bucket
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let state = AppState::connect(config).await.unwrap();
    oppor_backend::MIGRATOR.run(&state.db).await.unwrap();
    oppor_backend::worker::indexer::tick(&state).await.unwrap();
    let app = oppor_backend::http::router(state.clone());
    Harness {
        state,
        app,
        _node: node,
        accounts,
        token,
        schema,
        admin,
    }
}
async fn sign(h: &Harness, signer: Address, message: &str) -> String {
    h.state
        .rpc
        .request(
            "eth_sign",
            json!([signer, format!("0x{}", hex::encode(message.as_bytes()))]),
        )
        .await
        .unwrap()
        .as_str()
        .unwrap()
        .into()
}
async fn login(h: &Harness, wallet: Address, signer: Address) -> Session {
    let challenge = ok(
        h,
        "POST",
        "/auth/challenge",
        json!({"wallet":wallet,"chain_id":"5042"}),
        None,
    )
    .await;
    let message = challenge["message"].as_str().unwrap();
    let signature = sign(h, signer, message).await;
    let (status, body, headers) = response(
        &h.app,
        "POST",
        "/auth/verify",
        json!({"message":message,"signature":signature}),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Login failed: {body}");
    Session {
        cookie: headers["set-cookie"]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .into(),
        csrf: body["csrf_token"].as_str().unwrap().into(),
        wallet,
        user_id: serde_json::from_value(body["user_id"].clone()).unwrap(),
    }
}
async fn execute(h: &Harness, prepared: &Value) {
    let from: Address = serde_json::from_value(prepared["expected_sender"].clone()).unwrap();
    let to: Address = serde_json::from_value(prepared["to"].clone()).unwrap();
    rpc_send(
        &h.state.rpc,
        from,
        Some(to),
        prepared["data"].as_str().unwrap().into(),
    )
    .await;
    oppor_backend::worker::indexer::tick(&h.state)
        .await
        .unwrap();
}
fn draft(
    h: &Harness,
    mode: &str,
    capacity: u32,
    start: DateTime<Utc>,
    cutoff: DateTime<Utc>,
) -> Value {
    json!({"title":"Orbit community campaign","description":"Manual social review and escrow rewards.","chain_id":"5042","reward":{"asset_kind":"ERC20","token_address":h.token,"amount_base_units":"5000000","token_id":"0","nft_inventory":[]},"distribution":{"mode":mode,"winner_count":if mode=="RAFFLE"{1}else{0},"capacity":capacity,"registration_limit":if capacity==0{100}else{capacity},"allocation_policy":"EQUAL_POOL","reward_per_recipient":null},"start_at":start,"cutoff_at":cutoff,"review_deadline":cutoff+Duration::hours(1),"claim_deadline":cutoff+Duration::hours(25),"refund_recipient":null,"tasks":[{"task_type":"X_REPOST","target_url":"https://x.com/example/status/123","instructions":"Submit the link to your repost.","required":true}]})
}
async fn wait_until(time: DateTime<Utc>) {
    let remaining = (time - Utc::now()).num_milliseconds();
    if remaining > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(remaining as u64 + 50)).await;
    }
}

#[tokio::test]
#[ignore = "Requires compose.test.yml, Anvil, and compiled Solidity fixtures; run scripts/test.sh"]
async fn campaign_to_claim_security_and_concurrency() {
    let h = harness().await;
    let creator = login(&h, h.accounts[0], h.accounts[0]).await;
    assert_eq!(
        ok(&h, "GET", "/me", json!({}), Some(&creator)).await["user_id"],
        json!(creator.user_id)
    );
    let alice = login(&h, h.accounts[1], h.accounts[1]).await;
    let bob = login(&h, h.accounts[2], h.accounts[2]).await;
    let attacker = login(&h, h.accounts[3], h.accounts[3]).await;
    let administrator = login(&h, h.accounts[9], h.accounts[9]).await;
    assert_eq!(
        response(&h.app, "GET", "/admin/stats", json!({}), None, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    for path in [
        "/admin/me",
        "/admin/stats",
        "/admin/users",
        "/admin/campaigns",
        "/admin/jobs",
        "/admin/audit-logs",
    ] {
        assert_eq!(
            response(&h.app, "GET", path, json!({}), Some(&creator), None)
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        ok(&h, "GET", "/admin/me", json!({}), Some(&administrator)).await["role"],
        "ADMIN"
    );

    // Replay and concurrent consumption: a successful challenge cannot create a second session.
    let challenge = ok(
        &h,
        "POST",
        "/auth/challenge",
        json!({"wallet":h.accounts[4],"chain_id":"5042"}),
        None,
    )
    .await;
    let signature = sign(&h, h.accounts[4], challenge["message"].as_str().unwrap()).await;
    let payload = json!({"message":challenge["message"],"signature":signature});
    let (a, b) = tokio::join!(
        response(&h.app, "POST", "/auth/verify", payload.clone(), None, None),
        response(&h.app, "POST", "/auth/verify", payload.clone(), None, None)
    );
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::UNAUTHORIZED]);
    assert_eq!(
        response(&h.app, "POST", "/auth/verify", payload, None, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );

    // Contract wallet signatures are verified by EIP-1271 rather than EOA recovery.
    let wallet = deploy(
        &h.state.rpc,
        h.accounts[0],
        "FixtureWallet",
        h.accounts[0].abi_encode(),
    )
    .await;
    let contract_session = login(&h, wallet, h.accounts[0]).await;
    assert_eq!(
        ok(&h, "GET", "/me", json!({}), Some(&contract_session)).await["wallet"],
        json!(wallet)
    );

    let now = DateTime::from_timestamp(Utc::now().timestamp(), 0).unwrap();
    let starts = now + Duration::seconds(12);
    let cutoff = now + Duration::seconds(24);
    let input = draft(&h, "RAFFLE", 2, starts, cutoff);
    let key = Uuid::new_v4().to_string();
    let (status, created, _) = response(
        &h.app,
        "POST",
        "/campaigns",
        input.clone(),
        Some(&creator),
        Some(&key),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let replay = response(
        &h.app,
        "POST",
        "/campaigns",
        input.clone(),
        Some(&creator),
        Some(&key),
    )
    .await;
    assert_eq!(replay.1, created);
    let mut conflicting = input.clone();
    conflicting["title"] = json!("Different campaign");
    assert_eq!(
        response(
            &h.app,
            "POST",
            "/campaigns",
            conflicting,
            Some(&creator),
            Some(&key)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let id: Uuid = serde_json::from_value(created["id"].clone()).unwrap();
    let base = format!("/campaigns/{id}");
    assert_eq!(
        response(&h.app, "GET", &base, json!({}), None, None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        response(
            &h.app,
            "POST",
            &format!("{base}/lock-config"),
            json!({"expected_version":0}),
            Some(&attacker),
            Some("attacker-key")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut bad_csrf = creator.clone();
    bad_csrf.csrf = "0".repeat(64);
    assert_eq!(
        response(
            &h.app,
            "POST",
            &format!("{base}/lock-config"),
            json!({"expected_version":0}),
            Some(&bad_csrf),
            Some("badcsrf-key")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let locked = ok(
        &h,
        "POST",
        &format!("{base}/lock-config"),
        json!({"expected_version":0}),
        Some(&creator),
    )
    .await;
    assert_eq!(
        response(
            &h.app,
            "PATCH",
            &base,
            json!({"expected_version":1,"campaign":input}),
            Some(&creator),
            Some("locked-edit-key")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert!(
        sqlx::query("UPDATE campaigns SET cutoff_at=cutoff_at+interval '1 second' WHERE id=$1")
            .bind(id)
            .execute(&h.state.db)
            .await
            .is_err()
    );
    let prep = ok(
        &h,
        "POST",
        &format!("{base}/prepare-create"),
        json!({}),
        Some(&creator),
    )
    .await;
    execute(&h, &prep).await;
    let c = oppor_backend::services::campaigns::load(&h.state, id)
        .await
        .unwrap();
    assert_eq!(c.status, "FUNDING");
    assert_eq!(
        json!(c.config_hash.as_ref().map(|b| B256::from_slice(b)).unwrap()),
        locked["config_hash"]
    );
    rpc_send(
        &h.state.rpc,
        creator.wallet,
        Some(h.token),
        format!(
            "0x{}",
            hex::encode(
                mintCall {
                    to: creator.wallet,
                    amount: U256::from(5_000_000)
                }
                .abi_encode()
            )
        ),
    )
    .await;
    let fund = ok(
        &h,
        "POST",
        &format!("{base}/prepare-fund"),
        json!({}),
        Some(&creator),
    )
    .await;
    assert_eq!(fund["fund"]["simulation"], "REQUIRES_CONFIRMED_APPROVAL");
    for a in fund["approvals"].as_array().unwrap() {
        execute(&h, a).await;
    }
    let fund = ok(
        &h,
        "POST",
        &format!("{base}/prepare-fund"),
        json!({}),
        Some(&creator),
    )
    .await;
    execute(&h, &fund["fund"]).await;
    let activate = ok(
        &h,
        "POST",
        &format!("{base}/prepare-activate"),
        json!({}),
        Some(&creator),
    )
    .await;
    execute(&h, &activate).await;
    wait_until(starts).await;
    let registered = ok(
        &h,
        "POST",
        &format!("{base}/entries"),
        json!({"x_username":"alice","discord_username":null}),
        Some(&alice),
    )
    .await;
    let a_entry: Uuid = serde_json::from_value(registered["id"].clone()).unwrap();
    moderation_checks(&h, &administrator, &creator, &alice, &bob, id).await;
    // A single remaining slot cannot be sold to two participants concurrently.
    let reg = json!({"x_username":null,"discord_username":null});
    let registration_path = format!("{base}/entries");
    let (a, b) = tokio::join!(
        response(
            &h.app,
            "POST",
            &registration_path,
            reg.clone(),
            Some(&bob),
            Some("race-bob-key")
        ),
        response(
            &h.app,
            "POST",
            &registration_path,
            reg,
            Some(&attacker),
            Some("race-other-key")
        )
    );
    let mut status = [a.0, b.0];
    status.sort();
    assert_eq!(status, [StatusCode::OK, StatusCode::CONFLICT]);
    let other = if a.0 == StatusCode::OK {
        &bob
    } else {
        &attacker
    };
    let other_entry: Uuid = serde_json::from_value(if a.0 == StatusCode::OK {
        a.1["id"].clone()
    } else {
        b.1["id"].clone()
    })
    .unwrap();
    let tasks = ok(&h, "GET", &base, json!({}), None).await["tasks"].clone();
    let task: Uuid = serde_json::from_value(tasks[0]["id"].clone()).unwrap();
    assert_eq!(
        response(
            &h.app,
            "GET",
            &format!("{base}/entries/{a_entry}/evidence"),
            json!({}),
            Some(&bob),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        response(
            &h.app,
            "POST",
            &format!("{base}/my-entry/submit"),
            json!({"expected_version":0}),
            Some(&alice),
            Some("missing-evidence-key")
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // Upload evidence using a size-bound signed PUT, then copy to an immutable private key.
    let png = hex::decode("89504e470d0a1a0a00000000").unwrap();
    let upload = ok(
        &h,
        "POST",
        "/uploads/presign",
        json!({"campaign_id":id,"content_type":"image/png","size_bytes":png.len()}),
        Some(&alice),
    )
    .await;
    assert_eq!(upload["upload"]["method"], "PUT");
    assert_eq!(upload["upload"]["size_bytes"], png.len());
    let signed_url = url::Url::parse(upload["upload"]["url"].as_str().unwrap()).unwrap();
    assert!(
        signed_url
            .query_pairs()
            .any(|(k, v)| k == "X-Amz-SignedHeaders" && v == "content-length;content-type;host")
    );
    let uploaded = reqwest::Client::new()
        .put(signed_url)
        .header("Content-Type", "image/png")
        .body(png.clone())
        .send()
        .await
        .unwrap();
    assert!(
        uploaded.status().is_success(),
        "Upload failed: {}",
        uploaded.text().await.unwrap()
    );
    let upload_id: Uuid = serde_json::from_value(upload["upload_id"].clone()).unwrap();
    use object_store::ObjectStoreExt;
    let staging: String = sqlx::query_scalar("SELECT staging_key FROM uploads WHERE id=$1")
        .bind(upload_id)
        .fetch_one(&h.state.db)
        .await
        .unwrap();
    let staging_key = object_store::path::Path::from(staging);
    for bad in [vec![0; 1], vec![0; png.len()]] {
        h.state
            .storage
            .client
            .put(&staging_key, bad.into())
            .await
            .unwrap();
        assert_eq!(
            response(
                &h.app,
                "POST",
                &format!("/uploads/{upload_id}/complete"),
                json!({}),
                Some(&alice),
                Some(&Uuid::new_v4().to_string())
            )
            .await
            .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    h.state
        .storage
        .client
        .put(&staging_key, png.clone().into())
        .await
        .unwrap();
    assert_eq!(
        response(
            &h.app,
            "POST",
            &format!("/uploads/{upload_id}/complete"),
            json!({}),
            Some(&creator),
            Some(&Uuid::new_v4().to_string())
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    ok(
        &h,
        "POST",
        &format!("/uploads/{upload_id}/complete"),
        json!({}),
        Some(&alice),
    )
    .await;
    let evidence_key: String = sqlx::query_scalar("SELECT object_key FROM uploads WHERE id=$1")
        .bind(upload_id)
        .fetch_one(&h.state.db)
        .await
        .unwrap();
    h.state
        .storage
        .client
        .put(&staging_key, vec![0; png.len()].into())
        .await
        .unwrap();
    assert_eq!(
        h.state
            .storage
            .read_evidence(&evidence_key, png.len() as u64, "image/png")
            .await
            .unwrap()
            .as_ref(),
        png.as_slice()
    );
    ok(&h,"PUT",&format!("{base}/my-entry/submissions/{task}"),json!({"expected_version":0,"text":"I completed this task.","url":"https://x.com/alice/status/1","upload_id":upload_id}),Some(&alice)).await;
    ok(
        &h,
        "POST",
        &format!("{base}/my-entry/submit"),
        json!({"expected_version":1}),
        Some(&alice),
    )
    .await;
    ok(&h,"PUT",&format!("{base}/my-entry/submissions/{task}"),json!({"expected_version":0,"text":"Completed","url":"https://x.com/other/status/2","upload_id":null}),Some(other)).await;
    ok(
        &h,
        "POST",
        &format!("{base}/my-entry/submit"),
        json!({"expected_version":1}),
        Some(other),
    )
    .await;
    assert_eq!(
        response(
            &h.app,
            "POST",
            &format!("{base}/entries/{a_entry}/review"),
            json!({"expected_version":2,"decision":"ELIGIBLE","reason":"Checked manually"}),
            Some(&creator),
            Some("early-review-key")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    wait_until(cutoff).await;
    h.state
        .rpc
        .request(
            "evm_setNextBlockTimestamp",
            json!([Utc::now().timestamp() + 1]),
        )
        .await
        .unwrap();
    h.state.rpc.request("evm_mine", json!([])).await.unwrap();
    assert_eq!(
        response(
            &h.app,
            "PUT",
            &format!("{base}/my-entry/submissions/{task}"),
            json!({"expected_version":2,"text":"Late edit","url":null,"upload_id":null}),
            Some(&alice),
            Some("late-evidence-key")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let campaign = oppor_backend::services::campaigns::load(&h.state, id)
        .await
        .unwrap();
    assert_eq!(
        response(
            &h.app,
            "POST",
            &format!("{base}/lock-eligibility"),
            json!({"expected_version":campaign.version}),
            Some(&creator),
            Some("pending-reviews-key")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    for e in [a_entry, other_entry] {
        ok(&h,"POST",&format!("{base}/entries/{e}/review"),json!({"expected_version":2,"decision":"ELIGIBLE","reason":"Evidence checked manually after cutoff."}),Some(&creator)).await;
    }
    assert_eq!(
        response(
            &h.app,
            "POST",
            &format!("{base}/entries/{a_entry}/review"),
            json!({"expected_version":2,"decision":"DISQUALIFIED","reason":"Stale decision"}),
            Some(&creator),
            Some("stale-review-key")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let campaign = oppor_backend::services::campaigns::load(&h.state, id)
        .await
        .unwrap();
    ok(
        &h,
        "POST",
        &format!("{base}/lock-eligibility"),
        json!({"expected_version":campaign.version}),
        Some(&creator),
    )
    .await;
    oppor_backend::worker::jobs::tick(&h.state, Uuid::new_v4())
        .await
        .unwrap();
    let preview = ok(
        &h,
        "GET",
        &format!("{base}/allocation-preview"),
        json!({}),
        Some(&creator),
    )
    .await;
    assert_eq!(preview["leaf_count"], 1);
    assert_eq!(preview["allocated_quantity"], "5000000");
    let transcript: Value =
        sqlx::query_scalar("SELECT transcript FROM raffles WHERE campaign_id=$1")
            .bind(id)
            .fetch_one(&h.state.db)
            .await
            .unwrap();
    let snapshot: Vec<u8> =
        sqlx::query_scalar("SELECT snapshot_hash FROM eligibility_snapshots WHERE campaign_id=$1")
            .bind(id)
            .fetch_one(&h.state.db)
            .await
            .unwrap();
    let entries=sqlx::query("SELECT entry_id,payout_wallet FROM snapshot_entries WHERE snapshot_id=(SELECT id FROM eligibility_snapshots WHERE campaign_id=$1) ORDER BY ordinal").bind(id).fetch_all(&h.state.db).await.unwrap();
    let pairs: Vec<(Uuid, Address)> = entries
        .iter()
        .map(|r| {
            (
                r.get("entry_id"),
                Address::from_slice(&r.get::<Vec<u8>, _>("payout_wallet")),
            )
        })
        .collect();
    let c: Campaign = oppor_backend::services::campaigns::load(&h.state, id)
        .await
        .unwrap();
    let draw = oppor_backend::services::allocations::build(
        &c,
        B256::from_slice(&snapshot),
        &pairs,
        Some(serde_json::from_value(transcript["seed"].clone()).unwrap()),
    )
    .unwrap();
    assert_eq!(json!(draw.root), preview["root"]);
    // Golden vector crosses Rust -> real Solidity ABI/hash, not a second Rust implementation.
    let allocation = &draw.allocations[0];
    let call = claimLeafCall {
        allocation: FixtureClaim {
            index: U256::ZERO,
            recipient: allocation.recipient,
            tokenId: U256::ZERO,
            quantity: U256::from(5_000_000),
        },
    };
    assert_eq!(
        h.state
            .rpc
            .call(c.escrow().unwrap(), call.abi_encode())
            .await
            .unwrap(),
        draw.leaves[0].as_slice()
    );
    let prepared = ok(
        &h,
        "POST",
        &format!("{base}/prepare-finalize"),
        json!({}),
        Some(&creator),
    )
    .await;
    execute(&h, &prepared).await;
    let winner = if allocation.recipient == alice.wallet {
        &alice
    } else {
        other
    };
    let loser = if winner.wallet == alice.wallet {
        other
    } else {
        &alice
    };
    let proofs = ok(
        &h,
        "GET",
        &format!("{base}/allocations/{}", winner.wallet),
        json!({}),
        None,
    )
    .await;
    assert_eq!(proofs["allocations"].as_array().unwrap().len(), 1);
    assert_eq!(
        response(
            &h.app,
            "POST",
            &format!("{base}/prepare-claim"),
            json!({"claim_index":0}),
            Some(loser),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    rpc_send(
        &h.state.rpc,
        creator.wallet,
        Some(h.token),
        format!(
            "0x{}",
            hex::encode(setPausedCall { value: true }.abi_encode())
        ),
    )
    .await;
    assert_eq!(
        response(
            &h.app,
            "POST",
            &format!("{base}/prepare-claim"),
            json!({"claim_index":0}),
            Some(winner),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM allocations WHERE campaign_id=$1 AND claimed_at IS NOT NULL",
    )
    .bind(id)
    .fetch_one(&h.state.db)
    .await
    .unwrap();
    assert_eq!(count, 0);
    rpc_send(
        &h.state.rpc,
        creator.wallet,
        Some(h.token),
        format!(
            "0x{}",
            hex::encode(setPausedCall { value: false }.abi_encode())
        ),
    )
    .await;
    let claim = ok(
        &h,
        "POST",
        &format!("{base}/prepare-claim"),
        json!({"claim_index":0}),
        Some(winner),
    )
    .await;
    execute(&h, &claim).await;
    let balance = h
        .state
        .rpc
        .uint(
            h.token,
            balanceOfCall {
                owner: winner.wallet,
            }
            .abi_encode(),
        )
        .await
        .unwrap();
    assert_eq!(balance, U256::from(5_000_000));
    assert_eq!(
        response(
            &h.app,
            "POST",
            &format!("{base}/prepare-claim"),
            json!({"claim_index":0}),
            Some(winner),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        response(
            &h.app,
            "POST",
            &format!("{base}/prepare-sweep"),
            json!({}),
            Some(&creator),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    oppor_backend::worker::indexer::tick(&h.state)
        .await
        .unwrap();
    let claimed: String = sqlx::query_scalar(
        "SELECT claimed_amount::text FROM campaign_rewards WHERE campaign_id=$1",
    )
    .bind(id)
    .fetch_one(&h.state.db)
    .await
    .unwrap();
    assert_eq!(claimed, "5000000");
    let tx: Vec<u8> =
        sqlx::query_scalar("SELECT claim_tx_hash FROM allocations WHERE campaign_id=$1")
            .bind(id)
            .fetch_one(&h.state.db)
            .await
            .unwrap();
    ok(
        &h,
        "POST",
        &format!("{base}/transactions"),
        json!({"tx_hash":B256::from_slice(&tx),"kind":"CLAIM"}),
        Some(winner),
    )
    .await;
    oppor_backend::worker::indexer::reconcile_transactions(&h.state)
        .await
        .unwrap();
    let tracked: String =
        sqlx::query_scalar("SELECT status FROM chain_transactions WHERE tx_hash=$1")
            .bind(&tx)
            .fetch_one(&h.state.db)
            .await
            .unwrap();
    assert_eq!(tracked, "CONFIRMED");
    // Replaying canonical logs is idempotent and cannot credit the same claim twice.
    sqlx::query("UPDATE indexer_cursors SET next_block=0,previous_block_hash=NULL")
        .execute(&h.state.db)
        .await
        .unwrap();
    oppor_backend::worker::indexer::tick(&h.state)
        .await
        .unwrap();
    let replayed: String = sqlx::query_scalar(
        "SELECT claimed_amount::text FROM campaign_rewards WHERE campaign_id=$1",
    )
    .bind(id)
    .fetch_one(&h.state.db)
    .await
    .unwrap();
    assert_eq!(replayed, "5000000");
    let results = ok(&h, "GET", &format!("{base}/results"), json!({}), None).await;
    assert!(results.get("evidence").is_none());
    assert!(results.get("raffle_transcript_url").is_some());
    let manifest = ok(&h, "GET", &format!("{base}/manifest"), json!({}), None).await;
    let bytes = reqwest::get(manifest["url"].as_str().unwrap())
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(json!(keccak256(bytes)), manifest["manifest_hash"]);
    // Domain bounds are enforced by PostgreSQL, independently of request validation.
    assert!(sqlx::query("UPDATE campaign_rewards SET funded_amount=115792089237316195423570985008687907853269984665640564039457584007913129639936 WHERE campaign_id=$1").bind(id).execute(&h.state.db).await.is_err());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM entries WHERE campaign_id=$1")
        .bind(id)
        .fetch_one(&h.state.db)
        .await
        .unwrap();
    assert_eq!(count, 2);
    ok(&h, "POST", "/auth/logout", json!({}), Some(&alice)).await;
    assert_eq!(
        response(&h.app, "GET", "/me", json!({}), Some(&alice), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    rpc_send(
        &h.state.rpc,
        creator.wallet,
        Some(wallet),
        format!(
            "0x{}",
            hex::encode(setEnabledCall { value: false }.abi_encode())
        ),
    )
    .await;
    assert_eq!(
        response(
            &h.app,
            "GET",
            "/me",
            json!({}),
            Some(&contract_session),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        response(&h.app, "GET", "/health/ready", json!({}), None, None)
            .await
            .0,
        StatusCode::OK
    );
    sqlx::query("UPDATE indexer_cursors SET updated_at=clock_timestamp()-interval '10 minutes'")
        .execute(&h.state.db)
        .await
        .unwrap();
    assert_eq!(
        response(&h.app, "GET", "/health/ready", json!({}), None, None)
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    oppor_backend::worker::indexer::tick(&h.state)
        .await
        .unwrap();
    assert_eq!(
        response(&h.app, "GET", "/health/ready", json!({}), None, None)
            .await
            .0,
        StatusCode::OK
    );
    let metrics = ok(&h, "GET", "/metrics", json!({}), None).await;
    assert!(
        metrics["body"]
            .as_str()
            .unwrap()
            .contains("oppor_indexer_last_verified_timestamp_seconds ")
    );
    assert!(
        !metrics["body"]
            .as_str()
            .unwrap()
            .contains("oppor_indexer_last_verified_timestamp_seconds 0\n")
    );
    administration_checks(&h, &administrator, &attacker, id).await;
    http_performance(&h).await;
    std::fs::write(
        "target/api-contract-responses.json",
        serde_json::to_vec_pretty(&*API_RECORDS.lock().unwrap()).unwrap(),
    )
    .unwrap();
    h.state.db.close().await;
    sqlx::query(&format!("DROP SCHEMA {} CASCADE", h.schema))
        .execute(&h.admin)
        .await
        .unwrap();
}

async fn moderation_checks(
    h: &Harness,
    admin: &Session,
    creator: &Session,
    participant: &Session,
    outsider: &Session,
    id: Uuid,
) {
    let path = format!("/admin/campaigns/{id}/moderation");
    let missing_csrf = Request::builder()
        .method("POST")
        .uri(format!("/v1{path}"))
        .header("origin", "http://localhost:3000")
        .header("content-type", "application/json")
        .header("cookie", &admin.cookie)
        .header("idempotency-key", "missing-admin-csrf-key")
        .body(Body::from(
            json!({"expected_version":0,"hidden":true,"reason":"Missing CSRF"}).to_string(),
        ))
        .unwrap();
    assert_eq!(
        h.app.clone().oneshot(missing_csrf).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let payload =
        json!({"expected_version":0,"hidden":true,"reason":"Investigating a community report."});
    assert_eq!(
        response(
            &h.app,
            "POST",
            &path,
            payload.clone(),
            Some(creator),
            Some("admin-denied-key")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let warm = ok(h, "GET", "/campaigns?limit=25", json!({}), None).await;
    assert!(
        warm["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == json!(id))
    );
    let key = Uuid::new_v4().to_string();
    let (status, hidden, _) = response(
        &h.app,
        "POST",
        &path,
        payload.clone(),
        Some(admin),
        Some(&key),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(hidden["version"], 1);
    assert_eq!(
        response(
            &h.app,
            "POST",
            &path,
            payload.clone(),
            Some(admin),
            Some(&key)
        )
        .await
        .1,
        hidden
    );
    assert_eq!(
        response(
            &h.app,
            "POST",
            &path,
            json!({"expected_version":0,"hidden":false,"reason":"Changed payload"}),
            Some(admin),
            Some(&key)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        response(
            &h.app,
            "POST",
            &path,
            payload,
            Some(admin),
            Some("stale-moderation-key")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert!(
        ok(h, "GET", "/campaigns?limit=25", json!({}), None).await["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        response(
            &h.app,
            "GET",
            &format!("/campaigns/{id}"),
            json!({}),
            None,
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        response(
            &h.app,
            "GET",
            &format!("/campaigns/{id}"),
            json!({}),
            Some(outsider),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    ok(
        h,
        "GET",
        &format!("/campaigns/{id}"),
        json!({}),
        Some(creator),
    )
    .await;
    ok(
        h,
        "GET",
        &format!("/campaigns/{id}"),
        json!({}),
        Some(participant),
    )
    .await;
    assert_eq!(
        response(
            &h.app,
            "POST",
            &format!("/campaigns/{id}/entries"),
            json!({}),
            Some(outsider),
            Some("hidden-registration-key")
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let hidden = ok(
        h,
        "GET",
        "/admin/campaigns?hidden=true",
        json!({}),
        Some(admin),
    )
    .await;
    assert_eq!(hidden["items"][0]["moderation"]["hidden"], true);
    ok(
        h,
        "POST",
        &path,
        json!({"expected_version":1,"hidden":false,"reason":"Review completed."}),
        Some(admin),
    )
    .await;
    assert!(
        !ok(h, "GET", "/campaigns?limit=25", json!({}), None).await["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let concurrent =
        json!({"expected_version":2,"hidden":true,"reason":"Concurrent moderation test."});
    let (first, second) = tokio::join!(
        response(
            &h.app,
            "POST",
            &path,
            concurrent.clone(),
            Some(admin),
            Some("concurrent-admin-one")
        ),
        response(
            &h.app,
            "POST",
            &path,
            concurrent,
            Some(admin),
            Some("concurrent-admin-two")
        )
    );
    let mut statuses = [first.0, second.0];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
    ok(
        h,
        "POST",
        &path,
        json!({"expected_version":3,"hidden":false,"reason":"Concurrency test complete."}),
        Some(admin),
    )
    .await;
}

async fn administration_checks(h: &Harness, admin: &Session, target: &Session, campaign: Uuid) {
    let stats = ok(h, "GET", "/admin/stats", json!({}), Some(admin)).await;
    assert_eq!(stats["counts"]["campaigns"], 1);
    assert_eq!(stats["counts"]["claimed_allocations"], 1);
    assert_eq!(stats["indexer"]["healthy"], true);
    let users = ok(h, "GET", "/admin/users?limit=1", json!({}), Some(admin)).await;
    assert_eq!(users["items"].as_array().unwrap().len(), 1);
    let next = users["next_cursor"].as_str().unwrap();
    let page = ok(
        h,
        "GET",
        &format!("/admin/users?limit=1&cursor={next}"),
        json!({}),
        Some(admin),
    )
    .await;
    assert_ne!(users["items"][0]["id"], page["items"][0]["id"]);
    assert!(users["items"][0].get("token_hash").is_none());
    assert_eq!(
        response(
            &h.app,
            "GET",
            "/admin/users?status=FAILED",
            json!({}),
            Some(admin),
            None
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let jobs = ok(
        h,
        "GET",
        "/admin/jobs?status=SUCCEEDED",
        json!({}),
        Some(admin),
    )
    .await;
    assert_eq!(jobs["items"][0]["status"], "SUCCEEDED");
    let failed_job = Uuid::new_v4();
    sqlx::query("INSERT INTO jobs(id,kind,dedupe_key,campaign_id,state,attempts,last_error) VALUES($1,'BUILD_ALLOCATIONS',$2,$3,'FAILED',8,'ALLOCATION_EXPIRED')")
        .bind(failed_job).bind(format!("admin-test:{failed_job}")).bind(campaign).execute(&h.state.db).await.unwrap();
    let failed = ok(
        h,
        "GET",
        "/admin/jobs?status=FAILED",
        json!({}),
        Some(admin),
    )
    .await;
    assert_eq!(failed["items"][0]["id"], json!(failed_job));
    assert_eq!(failed["items"][0]["last_error"], "ALLOCATION_EXPIRED");
    let protected = format!("/admin/users/{}/suspension", admin.user_id);
    assert_eq!(
        response(
            &h.app,
            "POST",
            &protected,
            json!({"expected_version":0,"suspended":true,"reason":"Not allowed."}),
            Some(admin),
            Some("protected-admin-key")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let path = format!("/admin/users/{}/suspension", target.user_id);
    let result = ok(
        h,
        "POST",
        &path,
        json!({"expected_version":0,"suspended":true,"reason":"Investigating abuse."}),
        Some(admin),
    )
    .await;
    assert_eq!(result["revoked_sessions"], 1);
    assert_eq!(
        response(&h.app, "GET", "/me", json!({}), Some(target), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let challenge = ok(
        h,
        "POST",
        "/auth/challenge",
        json!({"wallet":target.wallet,"chain_id":"5042"}),
        None,
    )
    .await;
    let signature = sign(h, target.wallet, challenge["message"].as_str().unwrap()).await;
    let proof = json!({"message":challenge["message"],"signature":signature});
    assert_eq!(
        response(&h.app, "POST", "/auth/verify", proof.clone(), None, None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    ok(
        h,
        "POST",
        &path,
        json!({"expected_version":1,"suspended":false,"reason":"Review cleared the account."}),
        Some(admin),
    )
    .await;
    let (status, signed_in, headers) =
        response(&h.app, "POST", "/auth/verify", proof, None, None).await;
    assert_eq!(status, StatusCode::OK);
    let restored = Session {
        cookie: headers["set-cookie"]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned(),
        csrf: signed_in["csrf_token"].as_str().unwrap().to_owned(),
        wallet: target.wallet,
        user_id: target.user_id,
    };
    assert_eq!(
        response(&h.app, "GET", "/me", json!({}), Some(target), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    ok(h, "GET", "/me", json!({}), Some(&restored)).await;
    ok(
        h,
        "POST",
        &format!("/admin/users/{}/revoke-sessions", target.user_id),
        json!({"expected_version":2,"reason":"Operator requested reauthentication."}),
        Some(admin),
    )
    .await;
    assert_eq!(
        response(&h.app, "GET", "/me", json!({}), Some(&restored), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let logs = ok(
        h,
        "GET",
        &format!("/admin/audit-logs?campaign_id={campaign}"),
        json!({}),
        Some(admin),
    )
    .await;
    assert!(
        logs["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["action"] == "ADMIN_CAMPAIGN_MODERATED")
    );
    // A stale administrator session remains valid for ordinary user routes but cannot administer.
    sqlx::query(
        "UPDATE sessions SET created_at=clock_timestamp()-interval '16 minutes' WHERE user_id=$1",
    )
    .bind(admin.user_id)
    .execute(&h.state.db)
    .await
    .unwrap();
    let denied = response(&h.app, "GET", "/admin/stats", json!({}), Some(admin), None).await;
    assert_eq!(denied.0, StatusCode::FORBIDDEN);
    assert_eq!(denied.1["error"]["code"], "ADMIN_REAUTH_REQUIRED");
    ok(h, "GET", "/me", json!({}), Some(admin)).await;
}

async fn http_performance(h: &Harness) {
    use futures_util::{StreamExt, stream};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = h.app.clone();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async {
            let _ = stopped.await;
        })
        .await
        .unwrap();
    });
    let client = reqwest::Client::new();
    let url = format!("http://{address}/v1/campaigns?limit=25");
    assert!(client.get(&url).send().await.unwrap().status().is_success());
    let started = std::time::Instant::now();
    let mut latencies: Vec<f64> = stream::iter(0..200)
        .map(|_| {
            let client = client.clone();
            let url = url.clone();
            async move {
                let start = std::time::Instant::now();
                let response = client.get(url).send().await.unwrap();
                assert_eq!(response.status(), reqwest::StatusCode::OK);
                let body: Value = response.json().await.unwrap();
                assert!(
                    body["items"]
                        .as_array()
                        .is_some_and(|items| !items.is_empty())
                );
                start.elapsed().as_secs_f64() * 1_000.0
            }
        })
        .buffer_unordered(32)
        .collect()
        .await;
    let elapsed = started.elapsed().as_secs_f64();
    latencies.sort_by(f64::total_cmp);
    let report = json!({"profile":if cfg!(debug_assertions){"debug"}else{"release"},"transport":"loopback HTTP","requests":200,"concurrency":32,"listing":"one finalized campaign, warm cache","elapsed_seconds":elapsed,"requests_per_second":200.0/elapsed,"p50_ms":latencies[99],"p95_ms":latencies[189],"p99_ms":latencies[197]});
    std::fs::create_dir_all("target").unwrap();
    std::fs::write(
        "target/http-performance.json",
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("HTTP_PERFORMANCE {report}");
    stop.send(()).unwrap();
    server.await.unwrap();
}
