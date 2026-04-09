pub mod auth;
pub mod crypto;
pub mod db;
pub mod middleware;
pub mod routes;

pub struct AppState {
    pub db: db::DbPool,
    pub hmac_secret: Vec<u8>,
    pub encryption_key: [u8; 32],
    pub rate_limiter: middleware::RateLimitState,
}
