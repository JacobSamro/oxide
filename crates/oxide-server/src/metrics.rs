use once_cell::sync::Lazy;
use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    register_int_gauge_vec_with_registry, HistogramVec, IntCounterVec, IntGaugeVec, Registry,
};

pub struct Metrics {
    pub registry: Registry,
    pub meta_cache: IntCounterVec,     // labels: result=hit|miss|stale_hit|swr
    pub tarball_cache: IntCounterVec,  // labels: result
    pub upstream_requests: IntCounterVec, // labels: kind=metadata|tarball, status
    pub upstream_latency: HistogramVec,   // labels: kind
    pub coalesced: IntCounterVec,         // labels: kind
    pub active_meta_fetches: IntGaugeVec, // labels: uplink
    pub active_tarball_streams: IntGaugeVec,
    pub response_bytes: HistogramVec,     // labels: kind
    pub rate_limited: IntCounterVec,      // labels: uplink
    pub audit: IntCounterVec,             // labels: result
    pub audit_latency: HistogramVec,
    pub mem_cache_size: IntGaugeVec,      // labels: cache
    pub mem_cache_evictions: IntCounterVec,
}

pub static METRICS: Lazy<Metrics> = Lazy::new(|| {
    let registry = Registry::new();
    Metrics {
        meta_cache: register_int_counter_vec_with_registry!(
            "oxide_metadata_cache_total", "metadata cache outcomes", &["result"], registry
        ).unwrap(),
        tarball_cache: register_int_counter_vec_with_registry!(
            "oxide_tarball_cache_total", "tarball cache outcomes", &["result"], registry
        ).unwrap(),
        upstream_requests: register_int_counter_vec_with_registry!(
            "oxide_upstream_requests_total", "upstream requests", &["kind", "status"], registry
        ).unwrap(),
        upstream_latency: register_histogram_vec_with_registry!(
            "oxide_upstream_latency_seconds", "upstream latency", &["kind"],
            vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0], registry
        ).unwrap(),
        coalesced: register_int_counter_vec_with_registry!(
            "oxide_coalesced_total", "coalesced inflight requests", &["kind"], registry
        ).unwrap(),
        active_meta_fetches: register_int_gauge_vec_with_registry!(
            "oxide_active_metadata_fetches", "active upstream metadata fetches", &["uplink"], registry
        ).unwrap(),
        active_tarball_streams: register_int_gauge_vec_with_registry!(
            "oxide_active_tarball_streams", "active tarball streams", &["uplink"], registry
        ).unwrap(),
        response_bytes: register_histogram_vec_with_registry!(
            "oxide_response_bytes", "response size bytes", &["kind"],
            vec![1024.0, 16384.0, 65536.0, 262144.0, 1048576.0, 8388608.0, 33554432.0, 134217728.0],
            registry
        ).unwrap(),
        rate_limited: register_int_counter_vec_with_registry!(
            "oxide_upstream_rate_limited_total", "429 from upstream", &["uplink"], registry
        ).unwrap(),
        audit: register_int_counter_vec_with_registry!(
            "oxide_audit_total", "audit endpoint outcomes", &["result"], registry
        ).unwrap(),
        audit_latency: register_histogram_vec_with_registry!(
            "oxide_audit_latency_seconds", "audit latency", &["result"],
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.25, 1.0, 5.0], registry
        ).unwrap(),
        mem_cache_size: register_int_gauge_vec_with_registry!(
            "oxide_mem_cache_size_bytes", "memory cache size", &["cache"], registry
        ).unwrap(),
        mem_cache_evictions: register_int_counter_vec_with_registry!(
            "oxide_mem_cache_evictions_total", "memory cache evictions", &["cache"], registry
        ).unwrap(),
        registry,
    }
});

pub fn render() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let mut buf = Vec::new();
    let _ = encoder.encode(&METRICS.registry.gather(), &mut buf);
    String::from_utf8(buf).unwrap_or_default()
}
