use std::path::PathBuf;

fn main() {
    let cargo_path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let cargo_dir = PathBuf::from(cargo_path);

    let src = cargo_dir.join("res/terminal.slang");
    let dst = cargo_dir.join("src/gfx/detail/terminal.wgsl");

    println!("{:?}", cargo_dir);

    std::process::Command::new("slangc")
        .args([
            src.to_str().unwrap(),
            "-target",
            "wgsl",
            "-o",
            dst.to_str().unwrap(),
        ])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}
