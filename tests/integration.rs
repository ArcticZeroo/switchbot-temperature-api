use chrono::{Duration, Utc};
use std::sync::Arc;

#[test]
fn db_insert_and_query_roundtrip() {
    let db = switchbot_api::db::Database::open(":memory:").unwrap();
    let now = Utc::now();
    let device_id = "TEST_DEVICE";

    db.insert_reading(device_id, 22.5, 58, now).unwrap();
    db.insert_reading(device_id, 21.0, 62, now + Duration::hours(1))
        .unwrap();

    let readings = db
        .query_readings(
            device_id,
            now - Duration::minutes(1),
            now + Duration::hours(2),
        )
        .unwrap();

    assert_eq!(readings.len(), 2);
    assert!((readings[0].temperature - 22.5).abs() < f64::EPSILON);
    assert_eq!(readings[0].humidity, 58);
    assert!((readings[1].temperature - 21.0).abs() < f64::EPSILON);
    assert_eq!(readings[1].humidity, 62);
}

#[test]
fn db_query_respects_time_range() {
    let db = switchbot_api::db::Database::open(":memory:").unwrap();
    let now = Utc::now();
    let device_id = "TEST_DEVICE";

    db.insert_reading(device_id, 20.0, 50, now - Duration::hours(3))
        .unwrap();
    db.insert_reading(device_id, 21.0, 55, now - Duration::hours(1))
        .unwrap();
    db.insert_reading(device_id, 22.0, 60, now).unwrap();

    let readings = db
        .query_readings(device_id, now - Duration::hours(2), now + Duration::minutes(1))
        .unwrap();

    assert_eq!(readings.len(), 2);
    assert!((readings[0].temperature - 21.0).abs() < f64::EPSILON);
    assert!((readings[1].temperature - 22.0).abs() < f64::EPSILON);
}

#[test]
fn db_cleanup_old_readings() {
    let db = switchbot_api::db::Database::open(":memory:").unwrap();
    let now = Utc::now();
    let device_id = "TEST_DEVICE";

    db.insert_reading(device_id, 20.0, 50, now - Duration::days(200))
        .unwrap();
    db.insert_reading(device_id, 22.0, 60, now).unwrap();

    let deleted = db.cleanup_old_readings().unwrap();
    assert_eq!(deleted, 1);

    let readings = db
        .query_readings(device_id, now - Duration::days(365), now + Duration::hours(1))
        .unwrap();
    assert_eq!(readings.len(), 1);
}

#[test]
fn db_query_different_devices() {
    let db = switchbot_api::db::Database::open(":memory:").unwrap();
    let now = Utc::now();

    db.insert_reading("DEVICE_A", 22.0, 60, now).unwrap();
    db.insert_reading("DEVICE_B", 18.0, 70, now).unwrap();

    let readings_a = db
        .query_readings("DEVICE_A", now - Duration::hours(1), now + Duration::hours(1))
        .unwrap();
    assert_eq!(readings_a.len(), 1);
    assert!((readings_a[0].temperature - 22.0).abs() < f64::EPSILON);

    let readings_b = db
        .query_readings("DEVICE_B", now - Duration::hours(1), now + Duration::hours(1))
        .unwrap();
    assert_eq!(readings_b.len(), 1);
    assert!((readings_b[0].temperature - 18.0).abs() < f64::EPSILON);
}

fn make_test_app() -> axum::Router {
    switchbot_api::api::router(Arc::new(switchbot_api::api::AppState {
        db: Arc::new(switchbot_api::db::Database::open(":memory:").unwrap()),
        switchbot: switchbot_api::switchbot::SwitchBotClient::new("fake".into(), "fake".into()),
        device_id: "TEST".into(),
    }))
}

fn make_test_app_with_db(db: Arc<switchbot_api::db::Database>) -> axum::Router {
    switchbot_api::api::router(Arc::new(switchbot_api::api::AppState {
        db,
        switchbot: switchbot_api::switchbot::SwitchBotClient::new("fake".into(), "fake".into()),
        device_id: "TEST".into(),
    }))
}

#[tokio::test]
async fn api_health_endpoint() {
    use axum::body::Body;
    use http::Request;
    use tower::ServiceExt;

    let resp = make_test_app()
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn api_rejects_missing_api_key() {
    use axum::body::Body;
    use http::Request;
    use tower::ServiceExt;

    // Without auth, unauthenticated requests should now succeed (200) as long as params are valid
    let resp = make_test_app()
        .oneshot(
            Request::builder()
                .uri("/api/climate/history?start=2026-01-01T00:00:00Z&end=2026-01-02T00:00:00Z")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn api_rejects_excessive_time_range() {
    use axum::body::Body;
    use http::Request;
    use tower::ServiceExt;

    let resp = make_test_app()
        .oneshot(
            Request::builder()
                .uri("/api/climate/history?start=2026-01-01T00:00:00Z&end=2026-01-20T00:00:00Z")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn api_history_returns_readings() {
    use axum::body::Body;
    use http::Request;
    use tower::ServiceExt;

    let db = Arc::new(switchbot_api::db::Database::open(":memory:").unwrap());
    let now = Utc::now();
    db.insert_reading("TEST", 22.5, 58, now).unwrap();

    let start = (now - Duration::hours(1))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let end = (now + Duration::hours(1))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let resp = make_test_app_with_db(db)
        .oneshot(
            Request::builder()
                .uri(&format!(
                    "/api/climate/history?start={}&end={}",
                    start, end
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
}
