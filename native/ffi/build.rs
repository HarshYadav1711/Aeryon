use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let cpp_dsp = strip_extended_path(
        manifest_dir
            .join("..")
            .join("cpp-dsp")
            .canonicalize()
            .unwrap_or_else(|_| manifest_dir.join("..").join("cpp-dsp")),
    );

    println!(
        "cargo:rerun-if-changed={}",
        cpp_dsp.join("CMakeLists.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        cpp_dsp.join("src").join("dsp.cpp").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        cpp_dsp
            .join("include")
            .join("aeryon")
            .join("dsp.h")
            .display()
    );

    let dst = cmake::Config::new(&cpp_dsp)
        .define("AERYON_DSP_BUILD_TESTS", "OFF")
        .build();

    let lib_dir = dst.join("lib");
    let lib64_dir = dst.join("lib64");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    if lib64_dir.exists() {
        println!("cargo:rustc-link-search=native={}", lib64_dir.display());
    }
    // Multi-config generators (Visual Studio) install under lib/<Config>.
    for config in ["Release", "Debug", "RelWithDebInfo", "MinSizeRel"] {
        let candidate = lib_dir.join(config);
        if candidate.exists() {
            println!("cargo:rustc-link-search=native={}", candidate.display());
        }
    }

    println!("cargo:rustc-link-lib=static=aeryon_dsp");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "linux" || target_os == "android" {
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=dylib=m");
    } else if target_os == "macos" || target_os == "ios" {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if target_os == "windows" && target_env != "msvc" {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
}

/// MSVC / CMake mishandle Win32 extended-length paths (`\\?\…`) as source roots.
fn strip_extended_path(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(stripped) = text.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path
    }
}
