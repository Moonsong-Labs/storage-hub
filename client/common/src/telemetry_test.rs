//! Comprehensive test suite for telemetry module

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex as TokioMutex;
    use tokio::time::timeout;

    /// Mock backend for testing
    struct MockBackend {
        events: Arc<TokioMutex<Vec<serde_json::Value>>>,
        enabled: bool,
        fail_on_send: Arc<TokioMutex<bool>>,
        send_delay_ms: Arc<TokioMutex<u64>>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                events: Arc::new(TokioMutex::new(Vec::new())),
                enabled: true,
                fail_on_send: Arc::new(TokioMutex::new(false)),
                send_delay_ms: Arc::new(TokioMutex::new(0)),
            }
        }

        fn with_failure() -> Self {
            let mut backend = Self::new();
            backend.fail_on_send = Arc::new(TokioMutex::new(true));
            backend
        }

        fn with_delay(delay_ms: u64) -> Self {
            let mut backend = Self::new();
            backend.send_delay_ms = Arc::new(TokioMutex::new(delay_ms));
            backend
        }

        async fn get_events(&self) -> Vec<serde_json::Value> {
            self.events.lock().await.clone()
        }

        async fn clear_events(&self) {
            self.events.lock().await.clear();
        }
    }

    #[async_trait]
    impl TelemetryBackend for MockBackend {
        async fn send_batch(
            &self,
            events: Vec<serde_json::Value>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let delay = *self.send_delay_ms.lock().await;
            if delay > 0 {
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            if *self.fail_on_send.lock().await {
                return Err("Mock send failure".into());
            }

            let mut stored = self.events.lock().await;
            stored.extend(events);
            Ok(())
        }

        fn is_enabled(&self) -> bool {
            self.enabled
        }

        async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    #[tokio::test]
    async fn test_bounded_channel_prevents_resource_exhaustion() {
        let mock = Arc::new(MockBackend::with_delay(100)); // Slow backend
        let config = TelemetryConfig {
            buffer_size: 10,
            batch_size: 5,
            flush_interval_secs: 60,
            overflow_strategy: OverflowStrategy::DropNewest,
            ..Default::default()
        };

        let service = TelemetryService::new(
            "test-service".to_string(),
            Some("node-1".to_string()),
            mock.clone(),
            config,
        );

        // Send more events than buffer can hold
        for i in 0..20 {
            service.send_raw_event(&format!("event_{}", i), serde_json::json!({"index": i}));
        }

        // Check that events were dropped
        let metrics = service.metrics();
        assert!(
            metrics.events_dropped > 0,
            "Should have dropped events due to buffer overflow"
        );

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Verify buffer size was respected
        let events = mock.get_events().await;
        assert!(events.len() <= 10, "Should not exceed buffer size");
    }

    #[tokio::test]
    async fn test_batching_efficiency() {
        let mock = Arc::new(MockBackend::new());
        let config = TelemetryConfig {
            buffer_size: 100,
            batch_size: 10,
            flush_interval_secs: 60,
            ..Default::default()
        };

        let mut service =
            TelemetryService::new("test-service".to_string(), None, mock.clone(), config);

        // Send exactly 20 events (should create 2 batches)
        for i in 0..20 {
            service.send_raw_event(&format!("event_{}", i), serde_json::json!({"index": i}));
        }

        // Wait for batching
        tokio::time::sleep(Duration::from_millis(100)).await;

        let metrics = service.metrics();
        assert_eq!(metrics.events_sent, 20);
        assert_eq!(
            metrics.batches_sent, 2,
            "Should have sent exactly 2 batches"
        );

        service.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_flush_interval() {
        let mock = Arc::new(MockBackend::new());
        let config = TelemetryConfig {
            buffer_size: 100,
            batch_size: 100,        // Large batch size
            flush_interval_secs: 1, // 1 second flush
            ..Default::default()
        };

        let service = TelemetryService::new("test-service".to_string(), None, mock.clone(), config);

        // Send only 3 events (won't trigger batch)
        for i in 0..3 {
            service.send_raw_event(&format!("event_{}", i), serde_json::json!({"index": i}));
        }

        // Wait for flush interval
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Check events were sent despite not reaching batch size
        let events = mock.get_events().await;
        assert_eq!(events.len(), 3, "Should have flushed 3 events");
    }

    #[tokio::test]
    async fn test_graceful_shutdown_flushes_events() {
        let mock = Arc::new(MockBackend::new());
        let config = TelemetryConfig {
            buffer_size: 100,
            batch_size: 50,
            flush_interval_secs: 60, // Long interval
            ..Default::default()
        };

        let mut service =
            TelemetryService::new("test-service".to_string(), None, mock.clone(), config);

        // Send events that won't trigger batch
        for i in 0..7 {
            service.send_raw_event(&format!("event_{}", i), serde_json::json!({"index": i}));
        }

        // Shutdown should flush
        service.shutdown().await.unwrap();

        // Check all events were sent
        let events = mock.get_events().await;
        assert_eq!(
            events.len(),
            7,
            "Should have flushed all 7 events on shutdown"
        );
    }

    #[tokio::test]
    async fn test_retry_logic_for_guaranteed_events() {
        let mock = Arc::new(MockBackend::new());
        let fail_flag = mock.fail_on_send.clone();

        let config = TelemetryConfig {
            buffer_size: 100,
            batch_size: 5,
            flush_interval_secs: 1,
            max_retries: 3,
            ..Default::default()
        };

        let mut service =
            TelemetryService::new("test-service".to_string(), None, mock.clone(), config);

        // Create a guaranteed event
        let event = GeneralTelemetryEvent {
            base: service.create_base_event("critical_event"),
            data: serde_json::json!({"critical": true}),
        };

        // Make backend fail initially
        *fail_flag.lock().await = true;

        // Send event with Guaranteed strategy
        service.send_event(event);

        // Wait for first attempt
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Enable backend
        *fail_flag.lock().await = false;

        // Wait for retry
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Check event was eventually sent
        let events = mock.get_events().await;
        assert!(
            events.len() > 0,
            "Guaranteed event should be retried and sent"
        );

        service.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_best_effort_events_dropped_on_failure() {
        let mock = Arc::new(MockBackend::with_failure());
        let config = TelemetryConfig {
            buffer_size: 100,
            batch_size: 5,
            flush_interval_secs: 1,
            ..Default::default()
        };

        let mut service =
            TelemetryService::new("test-service".to_string(), None, mock.clone(), config);

        // Send best effort events
        for i in 0..5 {
            service.send_raw_event(&format!("event_{}", i), serde_json::json!({"index": i}));
        }

        // Wait for processing
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Check events were dropped
        let metrics = service.metrics();
        assert_eq!(
            metrics.events_dropped, 5,
            "Best effort events should be dropped on failure"
        );
        assert_eq!(metrics.backend_errors, 1, "Should record backend error");

        service.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_overflow_strategy_drop_oldest() {
        let mock = Arc::new(MockBackend::with_delay(500)); // Very slow backend
        let config = TelemetryConfig {
            buffer_size: 5,
            batch_size: 10,
            flush_interval_secs: 60,
            overflow_strategy: OverflowStrategy::DropOldest,
            ..Default::default()
        };

        let service = TelemetryService::new("test-service".to_string(), None, mock.clone(), config);

        // Fill buffer and overflow
        for i in 0..10 {
            service.send_raw_event(&format!("event_{}", i), serde_json::json!({"index": i}));
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let metrics = service.metrics();
        assert!(
            metrics.events_dropped > 0,
            "Should drop events with DropOldest strategy"
        );
    }

    #[tokio::test]
    async fn test_concurrent_event_sending() {
        let mock = Arc::new(MockBackend::new());
        let config = TelemetryConfig {
            buffer_size: 1000,
            batch_size: 50,
            flush_interval_secs: 1,
            ..Default::default()
        };

        let service = Arc::new(TelemetryService::new(
            "test-service".to_string(),
            None,
            mock.clone(),
            config,
        ));

        // Spawn multiple tasks sending events concurrently
        let mut handles = vec![];
        for task_id in 0..10 {
            let svc = service.clone();
            let handle = tokio::spawn(async move {
                for i in 0..10 {
                    svc.send_raw_event(
                        &format!("event_{}_{}", task_id, i),
                        serde_json::json!({"task": task_id, "index": i}),
                    );
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Wait for processing
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Verify all events were sent
        let events = mock.get_events().await;
        assert_eq!(events.len(), 100, "Should have sent all 100 events");
    }

    #[tokio::test]
    async fn test_shutdown_timeout() {
        let mock = Arc::new(MockBackend::with_delay(10000)); // Extremely slow
        let config = TelemetryConfig {
            buffer_size: 100,
            batch_size: 5,
            flush_interval_secs: 60,
            ..Default::default()
        };

        let mut service =
            TelemetryService::new("test-service".to_string(), None, mock.clone(), config);

        // Send events
        for i in 0..10 {
            service.send_raw_event(&format!("event_{}", i), serde_json::json!({"index": i}));
        }

        // Shutdown should complete within timeout even if backend is slow
        let shutdown_result = timeout(Duration::from_secs(15), service.shutdown()).await;

        assert!(
            shutdown_result.is_ok(),
            "Shutdown should complete within timeout"
        );
    }

    #[tokio::test]
    async fn test_service_name_attribution() {
        let mock = Arc::new(MockBackend::new());
        let config = TelemetryConfig::default();

        let mut service = TelemetryService::new(
            "my-special-service".to_string(),
            Some("node-123".to_string()),
            mock.clone(),
            config,
        );

        service.send_raw_event("test_event", serde_json::json!({"data": "test"}));

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        let events = mock.get_events().await;
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(event["service"], "my-special-service");
        assert_eq!(event["node_id"], "node-123");

        service.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_metrics_accuracy() {
        let mock = Arc::new(MockBackend::new());
        let config = TelemetryConfig {
            buffer_size: 100,
            batch_size: 5,
            flush_interval_secs: 60,
            ..Default::default()
        };

        let mut service =
            TelemetryService::new("test-service".to_string(), None, mock.clone(), config);

        // Send 15 events (3 batches)
        for i in 0..15 {
            service.send_raw_event(&format!("event_{}", i), serde_json::json!({"index": i}));
        }

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(200)).await;

        let metrics = service.metrics();
        assert_eq!(metrics.events_sent, 15);
        assert_eq!(metrics.batches_sent, 3);
        assert_eq!(metrics.events_dropped, 0);
        assert_eq!(metrics.events_failed, 0);
        assert_eq!(metrics.backend_errors, 0);

        service.shutdown().await.unwrap();
    }
}
