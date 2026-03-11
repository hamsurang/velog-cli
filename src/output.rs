//! Format-aware output helpers for compact/silent modes.
//!
//! Pretty mode output is handled by existing functions in handlers.rs.
//! This module only handles compact (JSON) and silent (suppressed) output.

use crate::cli::Format;
use crate::models::CompactError;
use serde::Serialize;

/// Emit structured data to stdout as minified JSON.
/// Used for query commands (post list, post show, auth status).
/// In silent mode, behaves the same as compact (queries always emit data).
pub fn emit_data<T: Serialize>(format: Format, value: &T) {
    match format {
        Format::Compact | Format::Silent => {
            let json = serde_json::to_string(value).expect("Failed to serialize output");
            println!("{json}");
        }
        Format::Pretty => {} // Pretty mode handled by caller
    }
}

/// Emit mutation result (URL) to stdout as JSON.
/// In silent mode, nothing is emitted.
pub fn emit_mutation_result(format: Format, url: &str) {
    match format {
        Format::Compact => {
            let result = crate::models::CompactMutationResult {
                url: url.to_string(),
            };
            let json = serde_json::to_string(&result).expect("Failed to serialize output");
            println!("{json}");
        }
        Format::Silent => {
            // No output in silent mode for mutations
        }
        Format::Pretty => {} // Pretty mode handled by caller
    }
}

/// Emit success message to stderr as JSON.
/// In silent mode, nothing is emitted.
pub fn emit_ok(format: Format, msg: &str) {
    match format {
        Format::Compact => {
            let message = crate::models::CompactMessage {
                status: "ok".to_string(),
                msg: msg.to_string(),
            };
            let json = serde_json::to_string(&message).expect("Failed to serialize output");
            eprintln!("{json}");
        }
        Format::Silent => {
            // No output in silent mode
        }
        Format::Pretty => {} // Pretty mode handled by caller
    }
}

/// Emit error to stderr as JSON. Used in main.rs error handler.
/// Both compact and silent emit errors (silent needs error info for debugging).
pub fn emit_error(format: Format, msg: &str, exit_code: i32) {
    match format {
        Format::Compact | Format::Silent => {
            let error = CompactError {
                error: msg.to_string(),
                exit_code,
            };
            let json = serde_json::to_string(&error).expect("Failed to serialize output");
            eprintln!("{json}");
        }
        Format::Pretty => {} // Pretty mode handled by caller
    }
}
