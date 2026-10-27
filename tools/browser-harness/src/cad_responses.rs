//! CAD capture response fields shared with the existing events.log format.

use serde::Serialize;

#[derive(Serialize)]
pub struct Resource {
    url: String,
    status: i64,
    #[serde(rename = "serviceWorker")]
    service_worker: bool,
}

pub fn resource(url: String, status: i64, service_worker: Option<bool>) -> Resource {
    Resource {
        url,
        status,
        service_worker: service_worker.unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn response_preserves_status_and_worker_origin() {
        let ordinary = super::resource("http://example/".into(), 404, None);
        let worker = super::resource("http://example/worker".into(), 201, Some(true));
        assert_eq!(
            serde_json::to_value(ordinary).unwrap(),
            serde_json::json!({
                "url": "http://example/", "status": 404, "serviceWorker": false
            })
        );
        assert_eq!(
            serde_json::to_value(worker).unwrap(),
            serde_json::json!({
                "url": "http://example/worker", "status": 201, "serviceWorker": true
            })
        );
    }
}
