//! Live API tests. All `#[ignore]` so they never run by accident.
//!
//! Run manually:
//!   cargo test --test live -- --ignored jwt_async --nocapture
//!
//! `send_sms_*` tests COST MONEY — keep gated.

use std::time::{SystemTime, UNIX_EPOCH};

use signalwire::{LookupKind, PhoneNumberAvailableQueryParams, PhoneNumberOwnedFilterParams, SignalWireClient, SignalWireError, SmsMessage, SubprojectQueryParams, UpdatePhoneNumberRequest};

fn creds() -> SignalWireClient {
    dotenvy::dotenv().ok();
    let space = std::env::var("SIGNALWIRE_SPACE_NAME").expect("SIGNALWIRE_SPACE_NAME");
    let project = std::env::var("SIGNALWIRE_PROJECT_ID").expect("SIGNALWIRE_PROJECT_ID");
    let key = std::env::var("SIGNALWIRE_API_KEY").expect("SIGNALWIRE_API_KEY");
    SignalWireClient::new(space, project, key).expect("client build")
}

#[tokio::test]
#[ignore]
async fn jwt_async() {
    let c = creds();
    let r = c.get_jwt().await.expect("jwt");
    assert!(!r.jwt_token.is_empty());
    assert!(!r.refresh_token.is_empty());
}

#[tokio::test]
#[ignore]
async fn phone_numbers_available_async() {
    let c = creds();
    let q = PhoneNumberAvailableQueryParams::new().build();
    let r = c.get_phone_numbers_available("US", &q).await.expect("available");
    println!("got {} available", r.phone_numbers_available.len());
}

#[tokio::test]
#[ignore]
async fn phone_numbers_owned_async() {
    let c = creds();
    let q = PhoneNumberOwnedFilterParams::new().build();
    let r = c.get_phone_numbers_owned(&q).await.expect("owned");
    println!("owned: {}", r.data.len());
}

#[tokio::test]
#[ignore]
async fn list_subprojects_async() {
    let c = creds();
    let q = SubprojectQueryParams::new().build();
    let r = c.list_subprojects(&q).await.expect("list");
    assert!(!r.accounts.is_empty());
}

/// COSTS MONEY — sends a real SMS.
#[tokio::test]
#[ignore]
async fn send_sms_costs_money() {
    dotenvy::dotenv().ok();
    let from = std::env::var("SIGNALWIRE_FROM_NUMBER").expect("SIGNALWIRE_FROM_NUMBER");
    let to = std::env::var("SIGNALWIRE_TO_NUMBER").expect("SIGNALWIRE_TO_NUMBER");
    let c = creds();
    let msg = SmsMessage {
        from,
        to,
        body: "signalwire-rs hello".into(),
    };
    let resp = c.send_sms(&msg).await.expect("send");
    println!("sid={} status={}", resp.sid, resp.get_status());

    // poll status once
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let s = c.get_message_status(&resp.sid).await.expect("status");
    println!("after 5s: {}", s.get_status());
}

/// CREATES + DELETES a real subproject.
#[tokio::test]
#[ignore]
async fn subproject_lifecycle() {
    let c = creds();
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let name = format!("test-subproject-{ts}");

    let created = c.create_subproject(&name).await.expect("create");
    assert_eq!(created.friendly_name, name);

    let fetched = c.get_subproject(&created.sid).await.expect("get");
    assert_eq!(fetched.sid, created.sid);

    let updated_name = format!("{name}-upd");
    let updated = c.update_subproject(&created.sid, &updated_name, None).await.expect("update");
    assert_eq!(updated.friendly_name, updated_name);

    // signalwire requires status=closed before delete
    c.update_subproject(&created.sid, &updated_name, Some("closed")).await.expect("close");
    c.delete_subproject(&created.sid).await.expect("delete");

    match c.get_subproject(&created.sid).await {
        Err(SignalWireError::NotFound(_)) => {}
        Ok(_) => panic!("subproject still exists"),
        Err(e) => panic!("unexpected: {e:?}"),
    }
}

#[tokio::test]
#[ignore]
async fn lookup_async() {
    dotenvy::dotenv().ok();
    let phone = std::env::var("SIGNALWIRE_TEST_PHONE_NUMBER").unwrap_or_else(|_| "+12065550100".into());
    let c = creds();

    let basic = c.lookup_phone_number(&phone).await.expect("basic");
    assert!(!basic.country_code.is_empty());

    let _carrier = c.lookup(&phone, LookupKind::Carrier).await.expect("carrier");
    let _cnam = c.lookup(&phone, LookupKind::CallerName).await.expect("cnam");
}

/// Updates routing on a phone number you own. Set both env vars to enable.
#[tokio::test]
#[ignore]
async fn update_phone_number_async() {
    dotenvy::dotenv().ok();
    let id = match std::env::var("SIGNALWIRE_TEST_PHONE_ID") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("set SIGNALWIRE_TEST_PHONE_ID to run");
            return;
        }
    };
    let c = creds();
    let mut req = UpdatePhoneNumberRequest::new();
    req.name = Some("rust-sdk-test".into());
    let resp = c.update_phone_number(&id, &req).await.expect("update");
    assert_eq!(resp.id, id);
}

#[cfg(feature = "blocking")]
#[test]
#[ignore]
fn jwt_blocking() {
    use signalwire::BlockingClient;
    dotenvy::dotenv().ok();
    let space = std::env::var("SIGNALWIRE_SPACE_NAME").expect("SIGNALWIRE_SPACE_NAME");
    let project = std::env::var("SIGNALWIRE_PROJECT_ID").expect("SIGNALWIRE_PROJECT_ID");
    let key = std::env::var("SIGNALWIRE_API_KEY").expect("SIGNALWIRE_API_KEY");
    let c = BlockingClient::new(space, project, key).expect("client build");
    let r = c.get_jwt().expect("jwt");
    assert!(!r.jwt_token.is_empty());
}
