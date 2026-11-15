use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=SUNREACTOR_GIT_HASH");

    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let dot_git = manifest_dir.join(".git");
    println!("cargo:rerun-if-changed={}", dot_git.join("HEAD").display());

    // Re-evaluate the `-dirty` marker when sources or the index change;
    // otherwise an edit after a clean build would keep reporting a clean hash.
    for path in ["src", "Cargo.toml", "Cargo.lock"] {
        println!(
            "cargo:rerun-if-changed={}",
            manifest_dir.join(path).display()
        );
    }

    let git_dir = resolve_git_dir(&dot_git);
    if let Some(git_dir) = git_dir.as_deref() {
        println!("cargo:rerun-if-changed={}", git_dir.join("index").display());
        if let Some(reference) = read_head_ref(git_dir) {
            println!(
                "cargo:rerun-if-changed={}",
                git_dir.join(reference).display()
            );
        }
    }

    let git_hash = env::var_os("SUNREACTOR_GIT_HASH")
        .map(|value| value.to_string_lossy().into_owned())
        .or_else(|| git_hash_from_checkout(&manifest_dir));
    if let Some(hash) = git_hash.as_deref() {
        println!("cargo:rustc-env=SUNREACTOR_GIT_HASH={hash}");
    }

    let package_version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| String::from("0.0.0"));
    let reported_version = match git_hash.as_deref().filter(|hash| !hash.is_empty()) {
        Some(hash) => format!("{package_version} ({hash})"),
        None => package_version,
    };
    println!("cargo:rustc-env=SUNREACTOR_VERSION={reported_version}");
}

fn git_hash_from_checkout(manifest_dir: &Path) -> Option<String> {
    let hash_output = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .current_dir(manifest_dir)
        .output()
        .ok()?;
    if !hash_output.status.success() {
        return None;
    }
    let mut hash = String::from_utf8(hash_output.stdout)
        .ok()?
        .trim_end()
        .to_owned();
    if hash.is_empty() {
        return None;
    }

    if git_is_dirty(manifest_dir) {
        hash.push_str("-dirty");
    }
    Some(hash)
}

fn resolve_git_dir(dot_git: &Path) -> Option<PathBuf> {
    let metadata = fs::metadata(dot_git).ok()?;
    if metadata.is_dir() {
        return Some(dot_git.to_owned());
    }
    if !metadata.is_file() {
        return None;
    }

    let contents = fs::read_to_string(dot_git).ok()?;
    let value = contents.strip_prefix("gitdir:")?.trim();
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Some(path)
    } else {
        dot_git.parent().map(|parent| parent.join(path))
    }
}

fn read_head_ref(git_dir: &Path) -> Option<String> {
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    head.strip_prefix("ref: ")
        .map(|reference| reference.trim().to_owned())
}

fn git_is_dirty(manifest_dir: &Path) -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(manifest_dir)
        .output()
        .map(|output| output.status.success() && !output.stdout.is_empty())
        .unwrap_or(false)
}
