// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Load .env file if it exists (for API keys)
    // This loads environment variables before the app starts
    if let Err(e) = dotenvy::dotenv() {
        // It's okay if .env doesn't exist - user might use UI to set keys
        eprintln!("Note: No .env file found ({}), using settings file or UI for API keys", e);
    } else {
        eprintln!("Loaded environment variables from .env file");
    }

    meetbetter_lib::run()
}
