use std::{env, path::Path, process::Command};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let app_dir = Path::new(&manifest_dir).join("app");

    println!("cargo:rerun-if-changed=app/package.json");
    println!("cargo:rerun-if-changed=app/package-lock.json");
    println!("cargo:rerun-if-changed=app/index.html");
    println!("cargo:rerun-if-changed=app/src");

    let status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(&app_dir)
        .status()
        .expect("failed to execute `npm run build`");

    if !status.success() {
        panic!("frontend build failed");
    }

    let index = app_dir.join("dist/index.html");

    if !index.exists() {
        panic!(
            "frontend build succeeded, but {} was not created",
            index.display()
        );
    }
}
