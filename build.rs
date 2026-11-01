use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let prefix = match env::var("PROJECTM_PREFIX") {
        Ok(p) => PathBuf::from(p),
        Err(_) => manifest.join("vendor/projectm4"),
    };
    let include = prefix.join("include");
    let lib = prefix.join("lib");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    let src = manifest.join("src/milkdrop_shim.cpp");
    let obj = out.join("milkdrop_shim.o");
    let archive = out.join("libmilkdrop_shim.a");

    println!("cargo:rerun-if-changed={}", src.display());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PROJECTM_PREFIX");

    let status = Command::new("clang++")
        .args([
            "-std=c++17",
            "-O2",
            "-fPIC",
            "-DGL_SILENCE_DEPRECATION",
            &format!("-I{}", include.display()),
            "-c",
            &src.display().to_string(),
            "-o",
            &obj.display().to_string(),
        ])
        .status()
        .expect("failed to run clang++");
    assert!(status.success(), "clang++ failed building milkdrop shim");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let macos_obj = out.join("macos_window.o");
    if target_os == "macos" {
        let macos_src = manifest.join("src/macos_window.mm");
        println!("cargo:rerun-if-changed={}", macos_src.display());
        let status = Command::new("clang++")
            .args([
                "-std=c++17",
                "-O2",
                "-fPIC",
                "-x",
                "objective-c++",
                "-c",
                &macos_src.display().to_string(),
                "-o",
                &macos_obj.display().to_string(),
            ])
            .status()
            .expect("failed to run clang++ for macos_window.mm");
        assert!(status.success(), "clang++ failed building macos window shim");
    }

    let _ = std::fs::remove_file(&archive);
    let mut ar = Command::new("ar");
    ar.arg("crs").arg(&archive).arg(&obj);
    if target_os == "macos" {
        ar.arg(&macos_obj);
    }
    let status = ar.status().expect("failed to run ar");
    assert!(status.success(), "ar failed creating milkdrop archive");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=milkdrop_shim");
    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=dylib=projectM-4");
    println!("cargo:rustc-link-lib=dylib=projectM-4-playlist");
    println!("cargo:rustc-link-lib=dylib=c++");
    println!("cargo:rustc-link-lib=framework=OpenGL");
    if target_os == "macos" {
        println!("cargo:rustc-link-lib=framework=Cocoa");
    }
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib.display());
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
}
