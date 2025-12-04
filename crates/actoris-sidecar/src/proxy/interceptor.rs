//! Traffic Interceptor - Core proxy logic for Actoris sidecar
//!
//! This module implements request/response interception with:
//! - Compute metering (CPU time, memory)
//! - Request/response hashing for verification
//! - DID attachment to requests
//! - Metrics collection for billing

use crate::metering::{ComputeMetrics, MeteringCollector};
use actoris_common::Result;
use bytes::Bytes;
use dashmap::DashMap;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};

/// Intercepted request metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedRequest {
    /// Unique request ID
    pub request_id: String,
    /// Actor DID making the request
    pub actor_did: String,
    /// Client DID (recipient)
    pub client_did: Option<String>,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request body hash
    pub body_hash: [u8; 32],
    /// Request timestamp
    pub timestamp: i64,
    /// Content length
    pub content_length: usize,
    /// Headers of interest
    pub headers: Vec<(String, String)>,
}

/// Intercepted response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedResponse {
    /// Request ID this responds to
    pub request_id: String,
    /// HTTP status code
    pub status_code: u16,
    /// Response body hash
    pub body_hash: [u8; 32],
    /// Response timestamp
    pub timestamp: i64,
    /// Content length
    pub content_length: usize,
    /// Compute metrics for this request
    pub compute_metrics: ComputeMetrics,
    /// Processing latency in microseconds
    pub latency_us: u64,
}

/// Request/Response pair for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestResponsePair {
    pub request: InterceptedRequest,
    pub response: Option<InterceptedResponse>,
    pub start_time: i64,
    pub end_time: Option<i64>,
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Listen address for the proxy
    pub listen_addr: String,
    /// Upstream service address
    pub upstream_addr: String,
    /// Actor DID for this sidecar
    pub actor_did: String,
    /// Maximum request body size (bytes)
    pub max_body_size: usize,
    /// Request timeout
    pub timeout_ms: u64,
    /// Enable detailed metering
    pub enable_metering: bool,
    /// Buffer size for verification queue
    pub verification_buffer_size: usize,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8080".to_string(),
            upstream_addr: "127.0.0.1:8000".to_string(),
            actor_did: "did:key:unknown".to_string(),
            max_body_size: 10 * 1024 * 1024, // 10MB
            timeout_ms: 30000,
            enable_metering: true,
            verification_buffer_size: 10000,
        }
    }
}

/// Traffic interceptor proxy
pub struct TrafficInterceptor {
    config: ProxyConfig,
    /// Active requests being tracked
    active_requests: Arc<DashMap<String, RequestResponsePair>>,
    /// Request counter for unique IDs
    request_counter: Arc<AtomicU64>,
    /// Metering collector
    metering: Arc<MeteringCollector>,
    /// Channel to send completed pairs for verification
    verification_tx: mpsc::Sender<RequestResponsePair>,
    /// Prometheus metrics
    metrics: Arc<ProxyMetrics>,
}

/// Prometheus metrics for the proxy
pub struct ProxyMetrics {
    pub requests_total: prometheus::IntCounter,
    pub requests_active: prometheus::IntGauge,
    pub request_duration_seconds: prometheus::Histogram,
    pub request_body_bytes: prometheus::Histogram,
    pub response_body_bytes: prometheus::Histogram,
    pub upstream_errors: prometheus::IntCounter,
    pub compute_pflops: prometheus::Counter,
}

impl ProxyMetrics {
    pub fn new() -> Self {
        Self {
            requests_total: prometheus::IntCounter::new(
                "actoris_proxy_requests_total",
                "Total requests processed",
            )
            .unwrap(),
            requests_active: prometheus::IntGauge::new(
                "actoris_proxy_requests_active",
                "Currently active requests",
            )
            .unwrap(),
            request_duration_seconds: prometheus::Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "actoris_proxy_request_duration_seconds",
                    "Request processing duration",
                )
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
            )
            .unwrap(),
            request_body_bytes: prometheus::Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "actoris_proxy_request_body_bytes",
                    "Request body size in bytes",
                )
                .buckets(prometheus::exponential_buckets(100.0, 10.0, 8).unwrap()),
            )
            .unwrap(),
            response_body_bytes: prometheus::Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "actoris_proxy_response_body_bytes",
                    "Response body size in bytes",
                )
                .buckets(prometheus::exponential_buckets(100.0, 10.0, 8).unwrap()),
            )
            .unwrap(),
            upstream_errors: prometheus::IntCounter::new(
                "actoris_proxy_upstream_errors_total",
                "Total upstream connection errors",
            )
            .unwrap(),
            compute_pflops: prometheus::Counter::new(
                "actoris_proxy_compute_pflops_total",
                "Total PFLOP-hours metered",
            )
            .unwrap(),
        }
    }

    pub fn register(&self, registry: &prometheus::Registry) -> Result<()> {
        registry.register(Box::new(self.requests_total.clone()))?;
        registry.register(Box::new(self.requests_active.clone()))?;
        registry.register(Box::new(self.request_duration_seconds.clone()))?;
        registry.register(Box::new(self.request_body_bytes.clone()))?;
        registry.register(Box::new(self.response_body_bytes.clone()))?;
        registry.register(Box::new(self.upstream_errors.clone()))?;
        registry.register(Box::new(self.compute_pflops.clone()))?;
        Ok(())
    }
}

impl Default for ProxyMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl TrafficInterceptor {
    /// Create a new traffic interceptor
    pub fn new(
        config: ProxyConfig,
        verification_tx: mpsc::Sender<RequestResponsePair>,
    ) -> Self {
        Self {
            config,
            active_requests: Arc::new(DashMap::new()),
            request_counter: Arc::new(AtomicU64::new(0)),
            metering: Arc::new(MeteringCollector::new()),
            verification_tx,
            metrics: Arc::new(ProxyMetrics::new()),
        }
    }

    /// Get the verification sender for external use
    pub fn verification_sender(&self) -> mpsc::Sender<RequestResponsePair> {
        self.verification_tx.clone()
    }

    /// Get metrics reference
    pub fn metrics(&self) -> Arc<ProxyMetrics> {
        self.metrics.clone()
    }

    /// Generate unique request ID
    fn generate_request_id(&self) -> String {
        let count = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let timestamp = chrono::Utc::now().timestamp_millis();
        format!("req-{}-{}", timestamp, count)
    }

    /// Hash request/response body
    fn hash_body(body: &[u8]) -> [u8; 32] {
        *blake3::hash(body).as_bytes()
    }

    /// Extract relevant headers
    fn extract_headers(headers: &hyper::HeaderMap) -> Vec<(String, String)> {
        let relevant = ["content-type", "x-actor-did", "x-client-did", "x-action-type"];
        headers
            .iter()
            .filter(|(name, _)| relevant.contains(&name.as_str().to_lowercase().as_str()))
            .map(|(name, value)| {
                (
                    name.to_string(),
                    value.to_str().unwrap_or("").to_string(),
                )
            })
            .collect()
    }

    /// Start the proxy server
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        let addr: SocketAddr = self.config.listen_addr.parse().map_err(|e| {
            actoris_common::ActorisError::Config(format!("Invalid listen address: {}", e))
        })?;

        let listener = TcpListener::bind(addr).await.map_err(|e| {
            actoris_common::ActorisError::Network(format!("Failed to bind: {}", e))
        })?;

        info!(addr = %addr, "Actoris sidecar proxy listening");

        loop {
            let (stream, remote_addr) = listener.accept().await.map_err(|e| {
                actoris_common::ActorisError::Network(format!("Accept failed: {}", e))
            })?;

            let io = TokioIo::new(stream);
            let interceptor = self.clone_for_request();

            tokio::spawn(async move {
                let service = service_fn(move |req| {
                    let interceptor = interceptor.clone();
                    async move { interceptor.handle_request(req, remote_addr).await }
                });

                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .await
                {
                    error!("Connection error: {:?}", err);
                }
            });
        }
    }

    /// Clone interceptor state for request handling
    fn clone_for_request(&self) -> Arc<TrafficInterceptorHandler> {
        Arc::new(TrafficInterceptorHandler {
            config: self.config.clone(),
            active_requests: self.active_requests.clone(),
            request_counter: self.request_counter.clone(),
            metering: self.metering.clone(),
            verification_tx: self.verification_tx.clone(),
            metrics: self.metrics.clone(),
        })
    }
}

/// Handler for individual requests (cloneable)
struct TrafficInterceptorHandler {
    config: ProxyConfig,
    active_requests: Arc<DashMap<String, RequestResponsePair>>,
    request_counter: Arc<AtomicU64>,
    metering: Arc<MeteringCollector>,
    verification_tx: mpsc::Sender<RequestResponsePair>,
    metrics: Arc<ProxyMetrics>,
}

impl Clone for TrafficInterceptorHandler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            active_requests: self.active_requests.clone(),
            request_counter: self.request_counter.clone(),
            metering: self.metering.clone(),
            verification_tx: self.verification_tx.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

impl TrafficInterceptorHandler {
    fn generate_request_id(&self) -> String {
        let count = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let timestamp = chrono::Utc::now().timestamp_millis();
        format!("req-{}-{}", timestamp, count)
    }

    async fn handle_request(
        &self,
        req: Request<Incoming>,
        remote_addr: SocketAddr,
    ) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
        let start = Instant::now();
        let request_id = self.generate_request_id();

        self.metrics.requests_total.inc();
        self.metrics.requests_active.inc();

        debug!(
            request_id = %request_id,
            method = %req.method(),
            path = %req.uri().path(),
            remote = %remote_addr,
            "Intercepting request"
        );

        // Start metering
        let metering_handle = if self.config.enable_metering {
            Some(self.metering.start_measurement(&request_id))
        } else {
            None
        };

        // Extract request metadata
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let headers = TrafficInterceptor::extract_headers(req.headers());

        // Get actor DID from header or config
        let actor_did = req
            .headers()
            .get("x-actor-did")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.config.actor_did.clone());

        let client_did = req
            .headers()
            .get("x-client-did")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Read and hash request body
        let (parts, body) = req.into_parts();
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                error!("Failed to read request body: {}", e);
                self.metrics.requests_active.dec();
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Full::new(Bytes::from("Failed to read request body")))
                    .unwrap());
            }
        };

        let body_hash = TrafficInterceptor::hash_body(&body_bytes);
        let content_length = body_bytes.len();
        self.metrics.request_body_bytes.observe(content_length as f64);

        // Create intercepted request record
        let intercepted_request = InterceptedRequest {
            request_id: request_id.clone(),
            actor_did,
            client_did,
            method: method.clone(),
            path: path.clone(),
            body_hash,
            timestamp: chrono::Utc::now().timestamp_millis(),
            content_length,
            headers,
        };

        // Store in active requests
        let pair = RequestResponsePair {
            request: intercepted_request,
            response: None,
            start_time: chrono::Utc::now().timestamp_millis(),
            end_time: None,
        };
        self.active_requests.insert(request_id.clone(), pair);

        // Forward to upstream
        let response = self.forward_to_upstream(parts, body_bytes).await;

        // Stop metering
        let compute_metrics = if let Some(handle) = metering_handle {
            self.metering.stop_measurement(handle)
        } else {
            ComputeMetrics::default()
        };

        // Update PFLOP counter
        self.metrics.compute_pflops.inc_by(compute_metrics.pflop_hours);

        let elapsed = start.elapsed();
        self.metrics.request_duration_seconds.observe(elapsed.as_secs_f64());

        // Process response
        let (status, response_body) = match response {
            Ok((status, body)) => (status, body),
            Err(e) => {
                error!(request_id = %request_id, "Upstream error: {}", e);
                self.metrics.upstream_errors.inc();
                self.metrics.requests_active.dec();

                // Remove from active requests
                self.active_requests.remove(&request_id);

                return Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Full::new(Bytes::from(format!("Upstream error: {}", e))))
                    .unwrap());
            }
        };

        let response_hash = TrafficInterceptor::hash_body(&response_body);
        let response_length = response_body.len();
        self.metrics.response_body_bytes.observe(response_length as f64);

        // Create intercepted response record
        let intercepted_response = InterceptedResponse {
            request_id: request_id.clone(),
            status_code: status.as_u16(),
            body_hash: response_hash,
            timestamp: chrono::Utc::now().timestamp_millis(),
            content_length: response_length,
            compute_metrics,
            latency_us: elapsed.as_micros() as u64,
        };

        // Update active request with response
        if let Some(mut pair) = self.active_requests.get_mut(&request_id) {
            pair.response = Some(intercepted_response);
            pair.end_time = Some(chrono::Utc::now().timestamp_millis());

            // Send to verification queue
            let completed_pair = pair.clone();
            drop(pair);

            if let Err(e) = self.verification_tx.try_send(completed_pair) {
                warn!(request_id = %request_id, "Failed to queue for verification: {}", e);
            }
        }

        // Remove from active requests
        self.active_requests.remove(&request_id);
        self.metrics.requests_active.dec();

        debug!(
            request_id = %request_id,
            status = %status,
            latency_ms = elapsed.as_millis(),
            "Request completed"
        );

        // Build response with attestation header
        let mut response = Response::builder()
            .status(status)
            .header("x-actoris-request-id", &request_id)
            .header("x-actoris-input-hash", hex::encode(&body_hash))
            .header("x-actoris-output-hash", hex::encode(&response_hash))
            .body(Full::new(response_body))
            .unwrap();

        Ok(response)
    }

    async fn forward_to_upstream(
        &self,
        parts: hyper::http::request::Parts,
        body: Bytes,
    ) -> std::result::Result<(StatusCode, Bytes), String> {
        use hyper_util::client::legacy::Client;
        use hyper_util::rt::TokioExecutor;

        let client: Client<_, Full<Bytes>> =
            Client::builder(TokioExecutor::new()).build_http();

        // Build upstream URL
        let uri = format!(
            "http://{}{}",
            self.config.upstream_addr,
            parts.uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/")
        );

        // Build request
        let mut builder = Request::builder()
            .method(parts.method)
            .uri(&uri);

        // Copy headers
        for (name, value) in parts.headers.iter() {
            if name != "host" {
                builder = builder.header(name, value);
            }
        }

        let upstream_req = builder
            .body(Full::new(body))
            .map_err(|e| format!("Failed to build request: {}", e))?;

        // Send request with timeout
        let timeout = Duration::from_millis(self.config.timeout_ms);
        let response = tokio::time::timeout(timeout, client.request(upstream_req))
            .await
            .map_err(|_| "Request timeout".to_string())?
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?
            .to_bytes();

        Ok((status, body))
    }
}

/// Builder for TrafficInterceptor
pub struct TrafficInterceptorBuilder {
    config: ProxyConfig,
    buffer_size: usize,
}

impl TrafficInterceptorBuilder {
    pub fn new() -> Self {
        Self {
            config: ProxyConfig::default(),
            buffer_size: 10000,
        }
    }

    pub fn listen_addr(mut self, addr: &str) -> Self {
        self.config.listen_addr = addr.to_string();
        self
    }

    pub fn upstream_addr(mut self, addr: &str) -> Self {
        self.config.upstream_addr = addr.to_string();
        self
    }

    pub fn actor_did(mut self, did: &str) -> Self {
        self.config.actor_did = did.to_string();
        self
    }

    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.config.timeout_ms = ms;
        self
    }

    pub fn enable_metering(mut self, enable: bool) -> Self {
        self.config.enable_metering = enable;
        self
    }

    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn build(self) -> (TrafficInterceptor, mpsc::Receiver<RequestResponsePair>) {
        let (tx, rx) = mpsc::channel(self.buffer_size);
        let interceptor = TrafficInterceptor::new(self.config, tx);
        (interceptor, rx)
    }
}

impl Default for TrafficInterceptorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_body() {
        let body1 = b"test body content";
        let body2 = b"test body content";
        let body3 = b"different content";

        let hash1 = TrafficInterceptor::hash_body(body1);
        let hash2 = TrafficInterceptor::hash_body(body2);
        let hash3 = TrafficInterceptor::hash_body(body3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_request_id_generation() {
        let (interceptor, _rx) = TrafficInterceptorBuilder::new().build();

        let id1 = interceptor.generate_request_id();
        let id2 = interceptor.generate_request_id();

        assert_ne!(id1, id2);
        assert!(id1.starts_with("req-"));
        assert!(id2.starts_with("req-"));
    }

    #[test]
    fn test_config_default() {
        let config = ProxyConfig::default();
        assert_eq!(config.listen_addr, "127.0.0.1:8080");
        assert_eq!(config.upstream_addr, "127.0.0.1:8000");
        assert!(config.enable_metering);
    }
}
