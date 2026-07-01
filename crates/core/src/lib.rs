//! Shared claude.ai client used by both the Claudeometer desktop app
//! (`src-tauri`) and the headless service (`crates/service`).
//!
//! Kept deliberately tiny and free of any GUI/Tauri dependency so it can be
//! linked into a minimal, fast-compiling, easy-to-cross-compile binary.

pub mod claude;

pub use claude::{fetch_claude_usage, UsageData, UsageWindow};
