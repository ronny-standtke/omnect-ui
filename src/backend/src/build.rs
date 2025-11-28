use std::io;
use std::path::Path;
use std::process::Command;

fn main() {
    // Tell Cargo to only rerun this build script if specific files change
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../ui/dist");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads");

    let git_short_rev = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let git_short_rev = git_short_rev.trim();

    println!("cargo:rustc-env=GIT_SHORT_REV={git_short_rev}");

    // Generate embedded static files from Vue dist folder
    generate_static_files().expect("Failed to generate static files");
}

fn generate_static_files() -> io::Result<()> {
    // Path to the Vue dist folder (relative to backend Cargo.toml)
    let ui_dist_path = Path::new("../ui/dist");

    if !ui_dist_path.exists() {
        panic!(
            "UI dist folder not found at {:?}. Please build the frontend first with: cd ../ui && pnpm run build",
            ui_dist_path
        );
    }

    static_files::resource_dir(ui_dist_path)
        .build()
        .expect("Failed to build static resources");

    Ok(())
}
