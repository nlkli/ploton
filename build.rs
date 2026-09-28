use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    let web_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("web");

    for path in [
        "package.json",
        "package-lock.json",
        "index.html",
        "vite.config.ts",
        "tsconfig.json",
        "src",
    ] {
        println!("cargo:rerun-if-changed=web/{path}");
    }

    // Install dependencies on a fresh checkout.
    if !web_dir.join("node_modules").exists() {
        npm(&web_dir, &["ci"]);
    }
    npm(&web_dir, &["run", "build"]);

    let index = web_dir.join("dist/index.html");
    assert!(
        index.exists(),
        "frontend build succeeded, but {} was not created",
        index.display()
    );
}

fn npm(dir: &Path, args: &[&str]) {
    let program = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap_or_else(|e| panic!("failed to run `npm {}`: {e}", args.join(" ")));
    assert!(status.success(), "`npm {}` failed", args.join(" "));
}

