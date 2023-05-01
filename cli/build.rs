fn main() {
    if cfg!(feature = "ui") && !cfg!(debug_assertions) {
        build_ui();
    }
}

fn build_ui() {
    println!("cargo:rerun-if-changed=../ui/src");

    let ok = std::process::Command::new("pnpm.cmd")
        .args(&["run", "build"])
        .current_dir("../ui/")
        .status()
        .expect("failed to build ui")
        .success();

    assert!(ok, "pnpm run build -- bad exit code")
}
