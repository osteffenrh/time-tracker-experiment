# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust command-line time tracking utility that allows users to track work periods with start/stop commands and generate reports. The application stores data in `~/.work_time_tracker.json` and ensures only one active period at a time.

## Common Commands

### Build and Development
- `cargo build` - Build the project in debug mode
- `cargo build --release` - Build optimized release version
- `cargo run -- <command>` - Run the application with arguments (e.g., `cargo run -- start`)
- `cargo test` - Run all tests
- `cargo check` - Check code without building
- `cargo clippy` - Run linter for code quality checks
- `cargo fmt` - Format code according to Rust standards

### Application Commands
- `start [±H:MM]` - Start time tracking with optional offset
- `stop [±H:MM]` - Stop time tracking with optional offset  
- `today` - Report tracked time for current day
- `week` - Report tracked time for current week
- `month` - Report tracked time for current month

## Code Architecture

The codebase follows a clean separation of concerns:

### Core Modules
- **`main.rs`** - CLI interface and argument parsing. Handles user input, command dispatch, and output formatting
- **`logic.rs`** - Business logic and data structures. Contains `TimeSheet` and `Period` structs, tracking operations, and time calculations
- **`storage.rs`** - Data persistence layer. Handles JSON serialization/deserialization to `~/.work_time_tracker.json`
- **`lib.rs`** - Library entry point exposing public modules

### Key Data Structures
- `TimeSheet` - Main state container with completed periods and optional active period
- `Period` - Represents a time interval with start/end UTC timestamps
- `ReportingPeriod` - Enum for compile-time safety when selecting report intervals

### Important Patterns
- All timestamps stored as UTC `DateTime<Utc>` for consistency
- Idempotent operations (start/stop can be called multiple times safely)
- Overlap calculation for time reporting across periods
- Optional time offset support for backdating entries
- Graceful handling of timezone conversions and edge cases

## Key Dependencies
- `chrono` - Date/time handling with timezone support
- `serde`/`serde_json` - JSON serialization for data persistence
- `dirs` - Cross-platform home directory detection