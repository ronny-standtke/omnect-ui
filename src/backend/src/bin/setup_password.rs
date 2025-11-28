//! CLI tool to set up a password file for development/testing
//!
//! Usage: cargo run --bin setup-password --features=mock -- <password>

use omnect_ui::services::auth::password::PasswordService;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <password>", args[0]);
        std::process::exit(1);
    }

    let password = &args[1];

    match PasswordService::store_or_update_password(password) {
        Ok(()) => {
            println!("Password file created successfully");
        }
        Err(e) => {
            eprintln!("Failed to create password file: {e:#}");
            std::process::exit(1);
        }
    }
}
