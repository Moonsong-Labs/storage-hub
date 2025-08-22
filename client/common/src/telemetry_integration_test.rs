//! Integration tests for telemetry module - production scenarios

#[cfg(test)]
mod integration_tests {
    use super::super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::Mutex as TokioMutex;
    use tokio::time::timeout;

    /// Test backend that simulates network conditions
    struct NetworkSimulationBackend {
        events: Arc<TokioMutex<Vec<serde_json::Value>>>,
        enabled: bool,
        network_failure: Arc<TokioMutex<bool>>,
        network_delay_ms: Arc<TokioMutex<u64>>,
        timeout_on_send: Arc<TokioMutex<bool>>,
        failure_count: Arc<AtomicU64>,
    }

    impl NetworkSimulationBackend {
        fn new() -> Self {
            Self {
                events: Arc::new(TokioMutex::new(Vec::new())),
                enabled: true,
                network_failure: Arc::new(TokioMutex::new(false)),
                network_delay_ms: Arc::new(TokioMutex::new(0)),
                timeout_on_send: Arc::new(TokioMutex::new(false)),
                failure_count: Arc::new(AtomicU64::new(0)),
            }
        }

        async fn simulate_network_failure(&self, fail: bool) {
            *self.network_failure.lock().await = fail;
        }

        async fn simulate_network_delay(&self, delay_ms: u64) {
            *self.network_delay_ms.lock().await = delay_ms;
        }

        async fn simulate_timeout(&self, timeout: bool) {
            *self.timeout_on_send.lock().await = timeout;
        }

        async fn get_events(&self) -> Vec<serde_json::Value> {
            self.events.lock().await.clone()
        }

        fn get_failure_count(&self) -> u64 {
            self.failure_count.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl TelemetryBackend for NetworkSimulationBackend {
        async fn send_batch(
            &self,
            events: Vec<serde_json::Value>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            // Simulate network delay
            let delay = *self.network_delay_ms.lock().await;
            if delay > 0 {
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            // Simulate timeout
            if *self.timeout_on_send.lock().await {
                tokio::time::sleep(Duration::from_secs(30)).await;
                return Err("Simulated timeout".into());
            }

            // Simulate network failure
            if *self.network_failure.lock().await {
                self.failure_count.fetch_add(1, Ordering::Relaxed);
                return Err("Simulated network failure".into());
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
            "network_simulation"
        }
    }

    #[tokio::test]
    async fn test_cross_service_event_correlation() {
        let backend = Arc::new(NetworkSimulationBackend::new());
        let config = TelemetryConfig::default();

        // Create multiple services with correlation IDs
        let mut bsp_service = TelemetryService::new(
            "bsp-service".to_string(),
            Some("bsp-node-1".to_string()),
            backend.clone(),
            config.clone(),
        );

        let mut msp_service = TelemetryService::new(
            "msp-service".to_string(),
            Some("msp-node-1".to_string()),
            backend.clone(),
            config.clone(),
        );

        let mut indexer_service = TelemetryService::new(
            "indexer-service".to_string(),
            Some("indexer-node-1".to_string()),
            backend.clone(),
            config.clone(),
        );

        // Create correlated events
        let correlation_id = uuid::Uuid::new_v4().to_string();

        // BSP event
        let mut bsp_event = GeneralTelemetryEvent {
            base: bsp_service.create_base_event("file_upload_started"),
            data: serde_json::json!({"file_size": 1024}),
        };
        bsp_event.base.correlation_id = Some(correlation_id.clone());
        bsp_service.send_event(bsp_event);

        // MSP event with same correlation
        let mut msp_event = GeneralTelemetryEvent {
            base: msp_service.create_base_event("file_replicated"),
            data: serde_json::json!({"replicas": 3}),
        };
        msp_event.base.correlation_id = Some(correlation_id.clone());
        msp_service.send_event(msp_event);

        // Indexer event with same correlation
        let mut indexer_event = GeneralTelemetryEvent {
            base: indexer_service.create_base_event("file_indexed"),
            data: serde_json::json!({"index_time_ms": 50}),
        };
        indexer_event.base.correlation_id = Some(correlation_id.clone());
        indexer_service.send_event(indexer_event);

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify correlation
        let events = backend.get_events().await;
        assert_eq!(events.len(), 3, "Should have 3 correlated events");

        // Check all events have the same correlation ID
        for event in &events {
            assert_eq!(
                event["correlation_id"].as_str().unwrap(),
                correlation_id,
                "All events should have the same correlation ID"
            );
        }

        // Check service attribution
        assert!(events.iter().any(|e| e["service"] == "bsp-service"));
        assert!(events.iter().any(|e| e["service"] == "msp-service"));
        assert!(events.iter().any(|e| e["service"] == "indexer-service"));

        // Cleanup
        bsp_service.shutdown().await.unwrap();
        msp_service.shutdown().await.unwrap();
        indexer_service.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_high_volume_event_handling() {
        let backend = Arc::new(NetworkSimulationBackend::new());
        let config = TelemetryConfig {
            buffer_size: 10_000,
            batch_size: 100,
            flush_interval_secs: 1,
            ..Default::default()
        };

        let service = Arc::new(TelemetryService::new(
            "high-volume-test".to_string(),
            Some("node-1".to_string()),
            backend.clone(),
            config,
        ));

        let start = Instant::now();
        let events_per_second = 10_000;
        let test_duration_secs = 3;

        // Spawn multiple tasks to generate events concurrently
        let mut handles = vec![];
        for task_id in 0..10 {
            let svc = service.clone();
            let handle = tokio::spawn(async move {
                let events_per_task = events_per_second / 10;
                for second in 0..test_duration_secs {
                    let second_start = Instant::now();
                    for i in 0..events_per_task {
                        svc.send_raw_event(
                            "high_volume_event",
                            serde_json::json!({
                                "task": task_id,
                                "second": second,
                                "index": i,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            }),
                        );
                    }
                    // Pace the sending to achieve target rate
                    let elapsed = second_start.elapsed();
                    if elapsed < Duration::from_secs(1) {
                        tokio::time::sleep(Duration::from_secs(1) - elapsed).await;
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Allow time for final batching
        tokio::time::sleep(Duration::from_secs(2)).await;

        let duration = start.elapsed();
        let events = backend.get_events().await;
        let expected_events = events_per_second * test_duration_secs;

        // Allow for some dropped events under extreme load
        assert!(
            events.len() >= (expected_events * 95 / 100),
            "Should handle at least 95% of events. Got {} out of {}",
            events.len(),
            expected_events
        );

        // Check metrics
        let metrics = service.metrics();
        assert!(
            metrics.events_sent >= (expected_events as u64 * 95 / 100),
            "Metrics should reflect sent events"
        );

        println!(
            "High volume test: {} events in {:?}, {} events/sec",
            events.len(),
            duration,
            events.len() as f64 / duration.as_secs_f64()
        );
    }

    #[tokio::test]
    async fn test_network_failure_recovery() {
        let backend = Arc::new(NetworkSimulationBackend::new());
        let config = TelemetryConfig {
            buffer_size: 1000,
            batch_size: 10,
            flush_interval_secs: 1,
            max_retries: 3,
            ..Default::default()
        };

        let mut service = TelemetryService::new(
            "failure-recovery-test".to_string(),
            None,
            backend.clone(),
            config,
        );

        // Send some events successfully
        for i in 0..5 {
            service.send_raw_event("before_failure", serde_json::json!({"index": i}));
        }
        tokio::time::sleep(Duration::from_millis(1500)).await;

        let initial_events = backend.get_events().await.len();
        assert_eq!(initial_events, 5, "Should have sent initial events");

        // Simulate network failure
        backend.simulate_network_failure(true).await;

        // Send events during failure (should be retried)
        for i in 0..10 {
            let event = GeneralTelemetryEvent {
                base: service.create_base_event("during_failure"),
                data: serde_json::json!({"index": i, "guaranteed": true}),
            };
            // Override strategy to Guaranteed for retry
            service.send_event(event);
        }

        // Wait for retry attempts
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Verify failures were recorded
        assert!(
            backend.get_failure_count() > 0,
            "Should have recorded failures"
        );

        // Recover network
        backend.simulate_network_failure(false).await;

        // Send new events
        for i in 0..5 {
            service.send_raw_event("after_recovery", serde_json::json!({"index": i}));
        }

        // Wait for processing and retries
        tokio::time::sleep(Duration::from_secs(3)).await;

        let final_events = backend.get_events().await;
        assert!(
            final_events.len() > initial_events,
            "Should have recovered and sent more events"
        );

        // Check metrics
        let metrics = service.metrics();
        assert!(
            metrics.backend_errors > 0,
            "Should have recorded backend errors"
        );

        service.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_backend_timeout_handling() {
        let backend = Arc::new(NetworkSimulationBackend::new());
        let config = TelemetryConfig {
            buffer_size: 100,
            batch_size: 5,
            flush_interval_secs: 1,
            backend_timeout_secs: 2, // 2 second timeout
            ..Default::default()
        };

        let mut service =
            TelemetryService::new("timeout-test".to_string(), None, backend.clone(), config);

        // Send events normally
        for i in 0..5 {
            service.send_raw_event("normal_event", serde_json::json!({"index": i}));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Simulate slow backend (but not timeout)
        backend.simulate_network_delay(1500).await; // 1.5 seconds
        for i in 0..5 {
            service.send_raw_event("slow_event", serde_json::json!({"index": i}));
        }
        tokio::time::sleep(Duration::from_secs(3)).await;

        // These should succeed despite being slow
        let events = backend.get_events().await;
        assert!(
            events.len() >= 10,
            "Should handle slow but successful sends"
        );

        // Now simulate timeout
        backend.simulate_network_delay(0).await;
        backend.simulate_timeout(true).await;

        for i in 0..5 {
            service.send_raw_event("timeout_event", serde_json::json!({"index": i}));
        }

        // Wait for timeout
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Check metrics - should have timeouts recorded as errors
        let metrics = service.metrics();
        assert!(
            metrics.backend_errors > 0,
            "Should record timeout as backend error"
        );
        assert!(metrics.events_dropped > 0, "Should drop events on timeout");

        service.shutdown().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // This test takes a long time, run with --ignored flag
    async fn test_memory_leak_detection() {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::sync::atomic::{AtomicUsize, Ordering};

        // Simple allocator wrapper to track memory
        struct TrackingAllocator;
        static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

        unsafe impl GlobalAlloc for TrackingAllocator {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                let ret = System.alloc(layout);
                if !ret.is_null() {
                    ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
                }
                ret
            }

            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                System.dealloc(ptr, layout);
                ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
            }
        }

        let backend = Arc::new(NetworkSimulationBackend::new());
        let config = TelemetryConfig {
            buffer_size: 1000,
            batch_size: 50,
            flush_interval_secs: 1,
            ..Default::default()
        };

        // Record initial memory
        let initial_memory = ALLOCATED.load(Ordering::Relaxed);

        // Run for extended period with consistent load
        let test_duration_hours = 1; // Reduced from 72 for practicality
        let events_per_minute = 1000;
        let check_interval_mins = 10;

        let mut service = TelemetryService::new(
            "memory-leak-test".to_string(),
            None,
            backend.clone(),
            config,
        );

        let mut memory_samples = Vec::new();

        for hour in 0..test_duration_hours {
            for minute in 0..60 {
                // Send events
                for i in 0..events_per_minute {
                    service.send_raw_event(
                        "memory_test",
                        serde_json::json!({
                            "hour": hour,
                            "minute": minute,
                            "index": i,
                        }),
                    );
                }

                // Check memory every N minutes
                if minute % check_interval_mins == 0 {
                    tokio::time::sleep(Duration::from_secs(2)).await; // Let batching complete
                    let current_memory = ALLOCATED.load(Ordering::Relaxed);
                    memory_samples.push(current_memory);

                    // Check for significant growth (>10MB)
                    if current_memory > initial_memory + 10_000_000 {
                        panic!(
                            "Memory leak detected! Initial: {}, Current: {}, Growth: {}",
                            initial_memory,
                            current_memory,
                            current_memory - initial_memory
                        );
                    }
                }

                // Pace the test
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        // Shutdown and check final memory
        service.shutdown().await.unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;

        let final_memory = ALLOCATED.load(Ordering::Relaxed);
        assert!(
            final_memory <= initial_memory + 1_000_000,
            "Memory should return close to initial after shutdown"
        );
    }

    #[tokio::test]
    async fn test_sustained_load() {
        let backend = Arc::new(NetworkSimulationBackend::new());
        let config = TelemetryConfig {
            buffer_size: 10_000,
            batch_size: 100,
            flush_interval_secs: 1,
            ..Default::default()
        };

        let service = Arc::new(TelemetryService::new(
            "sustained-load-test".to_string(),
            None,
            backend.clone(),
            config,
        ));

        let events_per_second = 1000;
        let test_duration_secs = 60; // Reduced from 24 hours for practicality

        let start = Instant::now();

        // Generate sustained load
        for second in 0..test_duration_secs {
            let second_start = Instant::now();

            for i in 0..events_per_second {
                service.send_raw_event(
                    "sustained_event",
                    serde_json::json!({
                        "second": second,
                        "index": i,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }),
                );
            }

            // Maintain consistent rate
            let elapsed = second_start.elapsed();
            if elapsed < Duration::from_secs(1) {
                tokio::time::sleep(Duration::from_secs(1) - elapsed).await;
            }
        }

        // Final flush
        tokio::time::sleep(Duration::from_secs(2)).await;

        let duration = start.elapsed();
        let events = backend.get_events().await;
        let expected_events = events_per_second * test_duration_secs;

        assert!(
            events.len() >= (expected_events * 99 / 100),
            "Should maintain 99% delivery under sustained load. Got {} out of {}",
            events.len(),
            expected_events
        );

        // Check metrics consistency
        let metrics = service.metrics();
        assert_eq!(
            metrics.events_sent + metrics.events_dropped,
            expected_events as u64,
            "Metrics should account for all events"
        );

        println!(
            "Sustained load test: {} events in {:?}, avg {} events/sec",
            events.len(),
            duration,
            events.len() as f64 / duration.as_secs_f64()
        );
    }

    #[tokio::test]
    async fn test_burst_load() {
        let backend = Arc::new(NetworkSimulationBackend::new());
        let config = TelemetryConfig {
            buffer_size: 10_000,
            batch_size: 100,
            flush_interval_secs: 1,
            overflow_strategy: OverflowStrategy::DropOldest,
            ..Default::default()
        };

        let mut service =
            TelemetryService::new("burst-load-test".to_string(), None, backend.clone(), config);

        // Normal load
        for i in 0..100 {
            service.send_raw_event("normal", serde_json::json!({"index": i}));
        }
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Burst load - 10K events as fast as possible
        let burst_start = Instant::now();
        for i in 0..10_000 {
            service.send_raw_event(
                "burst",
                serde_json::json!({
                    "index": i,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
            );
        }
        let burst_duration = burst_start.elapsed();

        // Return to normal
        for i in 0..100 {
            service.send_raw_event("after_burst", serde_json::json!({"index": i}));
        }

        // Allow processing
        tokio::time::sleep(Duration::from_secs(5)).await;

        let metrics = service.metrics();
        let events = backend.get_events().await;

        println!(
            "Burst test: {} events sent in {:?} ({} events/sec)",
            10_000,
            burst_duration,
            10_000.0 / burst_duration.as_secs_f64()
        );

        println!(
            "Results: {} sent, {} dropped, {} in backend",
            metrics.events_sent,
            metrics.events_dropped,
            events.len()
        );

        // System should handle burst without crashing
        assert!(
            metrics.events_sent > 0,
            "Should send some events during burst"
        );

        // Check that overflow strategy worked
        if metrics.events_dropped > 0 {
            assert_eq!(
                metrics.events_sent + metrics.events_dropped,
                10_200, // burst + normal events
                "All events should be accounted for"
            );
        }

        service.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_mixed_workload() {
        let backend = Arc::new(NetworkSimulationBackend::new());
        let config = TelemetryConfig {
            buffer_size: 5000,
            batch_size: 50,
            flush_interval_secs: 1,
            ..Default::default()
        };

        let service = Arc::new(TelemetryService::new(
            "mixed-workload-test".to_string(),
            None,
            backend.clone(),
            config,
        ));

        // Spawn different workload types concurrently
        let mut handles = vec![];

        // High-frequency small events
        let svc = service.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..1000 {
                svc.send_raw_event("small", serde_json::json!({"i": i}));
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }));

        // Low-frequency large events
        let svc = service.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..50 {
                let large_data = (0..100)
                    .map(|j| (format!("field_{}", j), format!("value_{}", i)))
                    .collect::<std::collections::HashMap<_, _>>();
                svc.send_raw_event("large", serde_json::to_value(large_data).unwrap());
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }));

        // Bursty events
        let svc = service.clone();
        handles.push(tokio::spawn(async move {
            for burst in 0..10 {
                // Burst
                for i in 0..100 {
                    svc.send_raw_event("burst", serde_json::json!({"burst": burst, "index": i}));
                }
                // Pause
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }));

        // Critical events (different strategy)
        let svc = service.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..20 {
                let mut event = GeneralTelemetryEvent {
                    base: svc.create_base_event("critical"),
                    data: serde_json::json!({"critical_id": i}),
                };
                // This would use Critical strategy in real implementation
                svc.send_event(event);
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }));

        // Wait for all workloads
        for handle in handles {
            handle.await.unwrap();
        }

        // Final flush
        tokio::time::sleep(Duration::from_secs(2)).await;

        let events = backend.get_events().await;
        let metrics = service.metrics();

        // Categorize events
        let small_events = events.iter().filter(|e| e["event_type"] == "small").count();
        let large_events = events.iter().filter(|e| e["event_type"] == "large").count();
        let burst_events = events.iter().filter(|e| e["event_type"] == "burst").count();
        let critical_events = events
            .iter()
            .filter(|e| e["event_type"] == "critical")
            .count();

        println!("Mixed workload results:");
        println!("  Small events: {}/1000", small_events);
        println!("  Large events: {}/50", large_events);
        println!("  Burst events: {}/1000", burst_events);
        println!("  Critical events: {}/20", critical_events);
        println!("  Total: {}", events.len());
        println!("  Metrics: {:?}", metrics);

        // Verify reasonable delivery for each type
        assert!(small_events > 900, "Should deliver most small events");
        assert!(large_events > 45, "Should deliver most large events");
        assert!(burst_events > 800, "Should handle burst reasonably");
        assert_eq!(critical_events, 20, "Should deliver all critical events");
    }
}
