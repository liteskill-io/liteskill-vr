//! Dev tool: fill a `.lsvr` database with the JSON fixture so the UI has
//! realistic data to design against. Build headless (no GUI deps):
//!
//!   cargo run --bin seed --no-default-features -- <db.lsvr> <fixture.json>
//!
//! Prefer `task dev:seed`, which resets src-tauri/project.lsvr and loads
//! fixtures/demo-project.json.

use std::path::Path;
use std::process::ExitCode;

use liteskill_vr_lib::db::Database;
use liteskill_vr_lib::fixture;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let db_path = args.next().unwrap_or_else(|| "project.lsvr".to_string());
    let fixture_path = args
        .next()
        .unwrap_or_else(|| "fixtures/demo-project.json".to_string());

    let raw = match std::fs::read_to_string(&fixture_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read fixture '{fixture_path}': {e}");
            return ExitCode::FAILURE;
        }
    };
    let doc = match serde_json::from_str(&raw) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Fixture '{fixture_path}' is not valid JSON: {e}");
            return ExitCode::FAILURE;
        }
    };

    let db = match Database::open_or_init(Path::new(&db_path), "Demo Project") {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to open '{db_path}': {e}");
            return ExitCode::FAILURE;
        }
    };

    match fixture::apply(&db, &doc) {
        Ok(stats) => {
            println!("Seeded {db_path} from {fixture_path}:");
            println!("  {stats:#?}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Seeding failed: {e}");
            ExitCode::FAILURE
        }
    }
}
