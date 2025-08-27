use serde::{Deserialize, Serialize};

/// Type-safe error categories for telemetry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCategory {
    Network,
    Timeout,
    Permission,
    Storage,
    Proof,
    Blockchain,
    Capacity,
    FileOperation,
    ForestOperation,
    Configuration,
}

impl ErrorCategory {
    /// Convert to string for telemetry events
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCategory::Network => "network_error",
            ErrorCategory::Timeout => "timeout_error",
            ErrorCategory::Permission => "permission_error",
            ErrorCategory::Storage => "storage_error",
            ErrorCategory::Proof => "proof_error",
            ErrorCategory::Blockchain => "blockchain_error",
            ErrorCategory::Capacity => "capacity_error",
            ErrorCategory::FileOperation => "file_operation_error",
            ErrorCategory::ForestOperation => "forest_operation_error",
            ErrorCategory::Configuration => "configuration_error",
        }
    }
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Trait for errors to self-categorize for telemetry
pub trait TelemetryErrorCategory {
    /// Returns the telemetry category for this error
    fn telemetry_category(&self) -> ErrorCategory;
}

/// Implementation for anyhow::Error
/// Attempts to categorize based on error message content with fallback to Configuration
impl TelemetryErrorCategory for anyhow::Error {
    fn telemetry_category(&self) -> ErrorCategory {
        let error_msg = self.to_string().to_lowercase();

        // Try to categorize based on error message content
        if error_msg.contains("timeout") || error_msg.contains("deadline") {
            ErrorCategory::Timeout
        } else if error_msg.contains("network")
            || error_msg.contains("connection")
            || error_msg.contains("peer")
        {
            ErrorCategory::Network
        } else if error_msg.contains("permission")
            || error_msg.contains("access")
            || error_msg.contains("unauthorized")
        {
            ErrorCategory::Permission
        } else if error_msg.contains("storage")
            || error_msg.contains("file")
            || error_msg.contains("directory")
        {
            ErrorCategory::Storage
        } else if error_msg.contains("proof") || error_msg.contains("merkle") {
            ErrorCategory::Proof
        } else if error_msg.contains("blockchain")
            || error_msg.contains("extrinsic")
            || error_msg.contains("transaction")
        {
            ErrorCategory::Blockchain
        } else if error_msg.contains("capacity") || error_msg.contains("full") {
            ErrorCategory::Capacity
        } else if error_msg.contains("forest") {
            ErrorCategory::ForestOperation
        } else {
            // Default fallback for unknown errors
            ErrorCategory::Configuration
        }
    }
}
