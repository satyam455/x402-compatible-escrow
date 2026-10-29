use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    mandates: Arc<Mutex<HashMap<String, Mandate>>>,
    reservations: Arc<Mutex<HashMap<Uuid, Reservation>>>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Mandate {
    agent_id: String,
    agent_wallet: String,
    owner: String,
    per_tx_limit: u64,
    daily_limit: u64,
    approved_recipients: HashSet<String>,
    approved_mints: HashSet<String>,
    approved_networks: HashSet<String>,
    categories: HashSet<String>,
    starts_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    active: bool,
    spent_today: u64,
}
#[derive(Deserialize)]
struct AuthorizeRequest {
    request_id: Uuid,
    agent_id: String,
    buyer_wallet: String,
    recipient: String,
    amount: u64,
    mint: String,
    network: String,
    category: String,
    resource_id: String,
    expires_at: DateTime<Utc>,
}
#[derive(Serialize)]
struct Verdict {
    verdict_id: Uuid,
    request_id: Uuid,
    decision: &'static str,
    reason_code: &'static str,
    reason: String,
    expires_at: DateTime<Utc>,
    reservation_id: Option<Uuid>,
}
#[derive(Clone)]
struct Reservation {
    request_id: Uuid,
    agent_id: String,
    amount: u64,
    status: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let state = AppState {
        mandates: Arc::new(Mutex::new(HashMap::new())),
        reservations: Arc::new(Mutex::new(HashMap::new())),
    };
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/authorize", post(authorize))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 8787)))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn authorize(
    State(state): State<AppState>,
    Json(req): Json<AuthorizeRequest>,
) -> (StatusCode, Json<Verdict>) {
    let verdict_id = Uuid::new_v4();
    let now = Utc::now();
    let mandates = state.mandates.lock().unwrap();
    let Some(m) = mandates.get(&req.agent_id) else {
        return deny(
            verdict_id,
            &req,
            "AGENT_NOT_FOUND",
            "No mandate exists for this agent",
        );
    };
    let common = |code: &'static str, reason: String| {
        (
            StatusCode::OK,
            Json(Verdict {
                verdict_id,
                request_id: req.request_id,
                decision: "DENY",
                reason_code: code,
                reason,
                expires_at: req.expires_at,
                reservation_id: None,
            }),
        )
    };
    if !m.active || now < m.starts_at || now >= m.expires_at {
        return common(
            "MANDATE_EXPIRED_OR_SUSPENDED",
            "Mandate is not currently active".into(),
        );
    }
    if req.expires_at <= now || req.expires_at > m.expires_at {
        return common(
            "AUTHORIZATION_EXPIRED",
            "Authorization expiry is outside the mandate window".into(),
        );
    }
    if req.amount > m.per_tx_limit {
        return common(
            "PER_TRANSACTION_LIMIT",
            "Amount exceeds the per-transaction limit".into(),
        );
    }
    if m.spent_today.saturating_add(req.amount) > m.daily_limit {
        return common(
            "DAILY_LIMIT",
            "Amount exceeds the remaining daily budget".into(),
        );
    }
    if !m.approved_recipients.contains(&req.recipient) {
        return common(
            "RECIPIENT_NOT_ALLOWED",
            "Recipient is not in the mandate allowlist".into(),
        );
    }
    if !m.approved_mints.contains(&req.mint) {
        return common("ASSET_NOT_ALLOWED", "Asset mint is not approved".into());
    }
    if !m.approved_networks.contains(&req.network) {
        return common("NETWORK_NOT_ALLOWED", "Network is not approved".into());
    }
    if !m.categories.contains(&req.category) {
        return common("CATEGORY_NOT_ALLOWED", "Category is not approved".into());
    }
    drop(mandates);
    let mut reservations = state.reservations.lock().unwrap();
    if reservations
        .values()
        .any(|r| r.request_id == req.request_id)
    {
        return common(
            "DUPLICATE_REQUEST",
            "Request ID has already been reserved".into(),
        );
    }
    let reservation_id = Uuid::new_v4();
    reservations.insert(
        reservation_id,
        Reservation {
            request_id: req.request_id,
            agent_id: req.agent_id.clone(),
            amount: req.amount,
            status: "reserved".into(),
        },
    );
    (
        StatusCode::OK,
        Json(Verdict {
            verdict_id,
            request_id: req.request_id,
            decision: "ALLOW",
            reason_code: "POLICY_MATCH",
            reason: "Mandate permits this payment".into(),
            expires_at: req.expires_at,
            reservation_id: Some(reservation_id),
        }),
    )
}
fn deny(
    verdict_id: Uuid,
    req: &AuthorizeRequest,
    code: &'static str,
    reason: &str,
) -> (StatusCode, Json<Verdict>) {
    (
        StatusCode::OK,
        Json(Verdict {
            verdict_id,
            request_id: req.request_id,
            decision: "DENY",
            reason_code: code,
            reason: reason.into(),
            expires_at: req.expires_at,
            reservation_id: None,
        }),
    )
}
