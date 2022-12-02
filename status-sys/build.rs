use std::env;
use std::env::set_current_dir;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn get_go_bin() -> String {
    if cfg!(target_family = "unix") {
        let output = String::from_utf8(
            Command::new("/usr/bin/which")
                .arg("go")
                .output()
                .map_err(|e| println!("cargo:warning=Couldn't find `which` command: {}", e))
                .expect("`which` command not found")
                .stdout,
        )
        .expect("which output couldnt be parsed");

        if output.is_empty() {
            println!("cargo:warning=Couldn't find go binary installed, please ensure that it is installed and/or withing the system paths");
            panic!("Couldn't find `go` binary installed");
        }
        output.trim().to_string()
    } else if cfg!(target_family = "windows") {
        "go".into()
    } else {
        panic!("OS not supported!");
    }
}

fn build_status_go_lib(go_bin: &str, project_dir: &Path) {
    // Build status-go static lib
    // build command taken from status-go make file:
    // https://github.com/status-im/status-go/blob/0f7c9f52d87b0ec13da4d7a52c5779fc0362a5ac/Makefile#L144-L155
    let out_dir: PathBuf = env::var_os("OUT_DIR").unwrap().into();
    let vendor_path = project_dir.join("vendor");

    set_current_dir(vendor_path).expect("Moving to vendor dir");
    create_dir_all("./library").expect("`./library` dir could not be created");

    let mut cmd = Command::new(go_bin);

    // cmd.arg("run")
    //     .arg("./cmd/library/")
    //     .stdout(Stdio::from(File::create("./library/main.go").unwrap()))
    //     .output()
    //     .expect("`./library/main.go` file could not be created");

    let mut file = File::create("./library/main.go").unwrap();
    let output = cmd
        .arg("run")
        .arg("./cmd/library/")
        .output()
        .expect("`./library/main.go` file could not be created");
    file.write_all(&output.stdout).unwrap();

    cmd.env("CGO_ENABLED", "1")
        .arg("build")
        .arg("-buildmode=c-archive")
        .arg("-o")
        .arg(out_dir.join("libstatus.a"))
        .arg("./library"); // /statusgo-lib; /library

    // Setting `GOCACHE=/tmp/` for crates.io job that builds documentation
    // when a crate is being published or updated.
    if std::env::var("DOCS_RS").is_ok() {
        cmd.env("GOCACHE", "/tmp/");
    }

    cmd.status()
        .map_err(|e| println!("cargo:warning=go build failed due to: {}", e))
        .unwrap();

    set_current_dir(project_dir).expect("Going back to project dir");
}

fn generate_bindgen_code() {
    let lib_dir: PathBuf = env::var_os("OUT_DIR").unwrap().into();

    println!("cargo:rustc-link-search={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=status");
    println!("cargo:rerun-if-changed=libstatus.h");

    // Generate status bindings with bindgen
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(format!("{}/{}", lib_dir.display(), "libstatus.h"))
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn main() {
    let go_bin = get_go_bin();

    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    build_status_go_lib(&go_bin, &project_dir);
    generate_bindgen_code();
}
