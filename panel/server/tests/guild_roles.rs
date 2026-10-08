mod common;
use common::*;

#[tokio::test]
async fn leader_approves_join_request_and_role_controls_claims() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for name in ["Leader", "Applicant"] {
        let (status, body) =
            t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username":name,"password":"password123"}))).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    let leader = t.login("Leader", "password123").await;
    let applicant = t.login("Applicant", "password123").await;
    let applicant_uuid = t.uuid("Applicant").await;
    let (_, server) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let server_id = server["server"]["id"].as_i64().unwrap();
    let (status, guild) =
        t.call("POST", "/api/v1/guilds", Some(&leader), Some(json!({"instance_id":"smp","name":"Pimps","tag":"PIMP"}))).await;
    assert_eq!(status, StatusCode::OK, "{guild}");
    let id = guild["id"].as_str().unwrap();
    let path = format!("/api/v1/guilds/{id}");

    assert_eq!(t.call("POST", &format!("{path}/requests"), Some(&applicant), Some(json!({"message":"Let me in"}))).await.0, StatusCode::OK);
    assert_eq!(t.call("GET", &format!("{path}/requests"), Some(&applicant), None).await.0, StatusCode::FORBIDDEN);
    let (_, requests) = t.call("GET", &format!("{path}/requests"), Some(&leader), None).await;
    assert_eq!(requests[0]["message"], "Let me in");
    assert_eq!(
        t.call("POST", &format!("{path}/requests/{applicant_uuid}/respond"), Some(&leader), Some(json!({"accept":true}))).await.0,
        StatusCode::OK
    );

    let (status, role) =
        t.call("POST", &format!("{path}/roles"), Some(&leader), Some(json!({"name":"Builder","can_claim":false,"can_post":true}))).await;
    assert_eq!(status, StatusCode::OK, "{role}");
    assert_eq!(
        t.call("PUT", &format!("{path}/members/{applicant_uuid}/role"), Some(&leader), Some(json!({"role":"Builder"}))).await.0,
        StatusCode::OK
    );
    let claim = json!({"server_id":server_id,"dimension":"minecraft:overworld","chunk_x":1,"chunk_z":1});
    assert_eq!(t.call("POST", &format!("{path}/claim"), Some(&applicant), Some(claim)).await.0, StatusCode::FORBIDDEN);
    let role_id = role["id"].as_i64().unwrap();
    assert_eq!(t.call("DELETE", &format!("{path}/roles/{role_id}"), Some(&leader), None).await.0, StatusCode::OK);
    let (_, members) = t.call("GET", &format!("{path}/members"), Some(&leader), None).await;
    assert_eq!(members.as_array().unwrap().iter().find(|m| m["uuid"] == applicant_uuid).unwrap()["role"], "member");
}
