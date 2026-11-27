//! Consolidated integration test harness for all tests that link `nservo`.
//!
//! Keeping these modules under a single `tests/integration/main.rs` binary
//! avoids linking a separate copy of `libnservo` + Servo for each test file.

mod bookmarks_tests;
mod command_tests;
mod completion_tests;
mod gpu_tests;
mod hints_tests;
mod history_tests;
mod inspector_tests;
mod keymap_tests;
mod load_state_tests;
mod scroll_tests;
mod search_tests;
mod session_tests;
mod start_page_tests;
mod tab_model_tests;
mod text_input_tests;
