use std::collections::HashMap;
use std::io::Error;
use std::path::PathBuf;
use std::process::Command;
use std::vec;

use camino::Utf8Path;
use clap::Parser;
use uniffi::{KotlinBindingGenerator, PythonBindingGenerator, SwiftBindingGenerator};

#[derive(Parser, Debug)]
struct Cli {
    #[clap(long, short)]
    library: Option<String>,

    #[clap(long)]
    mode: Option<String>,

    #[clap(long)]
    language: Option<String>,

    #[clap(short, long)]
    publish: Option<bool>,

    #[clap(long)]
    format: Option<bool>,

    #[clap(long)]
    out_dir: Option<PathBuf>,
    #[clap(
        long,
        default_value = "ubuntu@chat.ruzhila.cn:/var/www/chat/downloads/"
    )]
    publish_server: Option<String>,
}
fn main() {
    let args = Cli::parse();
    let publish = args.publish.unwrap_or(false);
    let publish_server = args.publish_server.clone().unwrap_or_default();
    let crate_name = std::env::var("CARGO_PKG_NAME")
        .unwrap_or("restsend_ffi".to_string())
        .replace("-", "_");
    let crate_name = crate_name.as_str();
    let language = args.language.clone().unwrap_or_default();
    let mode = bindgen_with_language(crate_name, &args).expect("bindgen failed");

    match language.as_str() {
        "swift" => {
            // only mac support xcframework
            if std::env::consts::OS != "macos" {
                println!("Only macos support xcframework build");
                return;
            }
            build_xcframework(crate_name, mode.clone());
            if publish {
                if mode == "release" {
                    publish_ios_pods(publish_server);
                } else {
                    println!("🔥 Only publish release mode");
                }
            }
        }
        "kotlin" => {
            build_android_aar(crate_name, mode.clone(), publish, publish_server)
                .expect("build aar failed");
        }
        _ => return,
    }
}

fn build_android_aar(
    crate_name: &str,
    mode: String,
    publish: bool,
    publish_server: String,
) -> Result<(), Error> {
    let cargo_toml = std::fs::read_to_string("crates/restsend/Cargo.toml").unwrap();
    let version = cargo_toml
        .lines()
        .find(|line| line.contains("version"))
        .unwrap()
        .split("\"")
        .nth(1)
        .unwrap();

    println!("Publish version: {}", version);

    let aar_name = format!("restsend-ffi-{}", version);

    let build_dir = tempdir::TempDir::new_in("", ".aarbuild").unwrap();
    println!(
        "Build aar in {:?} mode: {} filename: {}",
        build_dir.path(),
        mode,
        aar_name
    );
    println!("▸ Sync sources {:?}", build_dir.path());
    std::fs::create_dir_all(build_dir.path().join("jniLibs")).unwrap();
    std::fs::create_dir_all(build_dir.path().join("libs")).unwrap();
    std::fs::create_dir_all(
        build_dir
            .path()
            .join("src")
            .join("uniffi")
            .join("restsend_sdk"),
    )
    .unwrap();

    let mut librarys: Vec<String> = vec![];
    let targets = vec![
        "aarch64-linux-android",
        "armv7-linux-androideabi",
        "x86_64-linux-android",
    ];

    for target in targets {
        let library = format!("target/{}/{}/lib{}.so", target, mode.clone(), crate_name);
        if std::path::Path::new(&library).exists() {
            librarys.push(library);
        }
    }

    if librarys.len() == 0 {
        panic!("🔥 No library found run `cargo build --target aarch64-linux-android --target armv7-linux-androideabi --target x86_64-linux-android` first");
    }

    for library in librarys {
        println!("▸ Add library {}", library);
        std::fs::copy(
            &library,
            build_dir
                .path()
                .join("jniLibs")
                .join(library.split("/").last().unwrap()),
        )
        .unwrap();
    }
    // add kotlin library .kt file
    let kotlin_file = "kotlin/uniffi/restsend_sdk/restsend_sdk.kt";
    if std::path::Path::new(kotlin_file).exists() {
        println!("▸ Add kotlin file {}", kotlin_file);
        std::fs::copy(
            kotlin_file,
            build_dir
                .path()
                .join("src")
                .join("uniffi")
                .join("restsend_sdk")
                .join("restsend_sdk.kt"),
        )
        .unwrap();
    }

    let status = Command::new("jar")
        .args(&[
            "cvf",
            build_dir
                .path()
                .join("libs")
                .join(format!("{}.jar", aar_name))
                .to_str()
                .unwrap(),
            "-C",
            build_dir.path().join("jniLibs").to_str().unwrap(),
            ".",
        ])
        .status();

    match status {
        Ok(status) => {
            if !status.success() {
                println!("🔥 Build aar failed");
                panic!("jar failed");
            } else {
                println!("🎉 Build aar success");
            }
        }
        Err(e) => {
            panic!("jar failed: {}", e);
        }
    }
    if !publish {
        return Ok(());
    }
    if mode != "release" {
        println!("🔥 Only publish release mode");
        return Ok(());
    }
    let aar_target = build_dir
        .path()
        .join("libs")
        .join(format!("{}.jar", aar_name))
        .to_str()
        .unwrap()
        .to_string();

    let aar_size = std::fs::metadata(&aar_target).unwrap().len() as f64 / 1024.0 / 1024.0;
    println!("▸ Sync aar size: {} M", aar_size);

    let status = Command::new("rsync")
        .args(&[&aar_target, &publish_server])
        .status();
    match status {
        Ok(status) => {
            if !status.success() {
                println!("🔥 Upload aar failed {:?}", aar_target);
                panic!("rsync failed");
            } else {
                println!("🎉 Upload aar success {}", aar_target);
            }
        }
        Err(e) => {
            panic!("rsync failed: {}", e);
        }
    }
    Ok(())
}

fn build_xcframework(crate_name: &str, mode: String) {
    let xcframework_name = "restsendFFI";
    let build_dir = tempdir::TempDir::new_in("", ".xcbuild").unwrap();
    println!(
        "Build xcframework in {:?} mode: {} framename: {}",
        build_dir.path(),
        mode,
        xcframework_name
    );
    println!("▸ Sync sources");

    std::fs::create_dir_all(build_dir.path().join("Headers")).unwrap();
    std::fs::create_dir_all(build_dir.path().join("Modules")).unwrap();

    std::fs::copy(
        "swift/module.h",
        build_dir.path().join("Headers").join("module.h"),
    )
    .unwrap();

    let mut librarys: Vec<String> = vec![];
    let targets = vec![
        "aarch64-apple-ios-sim",
        "aarch64-apple-ios",
        "x86_64-apple-darwin",
        "x86_64-apple-ios",
    ];

    for target in targets {
        let library = format!("target/{}/{}/lib{}.a", target, mode.clone(), crate_name);
        if std::path::Path::new(&library).exists() {
            librarys.push(library);
        }
    }
    if librarys.len() == 0 {
        panic!("🔥 No library found run `cargo build --target aarch64-apple-ios-sim --target x86_64-apple-darwin` first");
    }

    let mut build_args = vec!["-create-xcframework".to_string()];

    for library in librarys {
        println!("▸ Add library {}", library);
        build_args.push("-library".to_string());
        build_args.push(library);
    }

    std::fs::remove_dir_all(std::path::Path::new(&format!(
        "swift/{}.xcframework",
        xcframework_name
    )))
    .unwrap_or_default();

    let out_xcframework = format!("swift/{}.xcframework", xcframework_name);
    for arg in vec![
        "-headers".to_string(),
        build_dir
            .path()
            .join("Headers")
            .to_str()
            .unwrap()
            .to_string(),
        "-output".to_string(),
        out_xcframework.clone(),
    ] {
        build_args.push(arg);
    }

    let status = Command::new("xcodebuild").args(build_args.clone()).status();
    match status {
        Ok(status) => {
            if !status.success() {
                println!("🔥 Build xcframework failed {:?}", build_args);
                panic!("xcodebuild failed");
            } else {
                println!("🎉 Build xcframework success {}", out_xcframework);
            }
        }
        Err(e) => {
            panic!("xcodebuild failed: {}", e);
        }
    }
}

fn publish_ios_pods(publish_server: String) {
    // Get version from `swift/RestsendSdk.podspec`
    let cargo_toml = std::fs::read_to_string("crates/restsend/Cargo.toml").unwrap();
    let version = cargo_toml
        .lines()
        .find(|line| line.contains("version"))
        .unwrap()
        .split("\"")
        .nth(1)
        .unwrap();

    println!("Publish version: {}", version);

    // update version in podspec
    let podspec = std::fs::read_to_string("swift/RestsendSdk.podspec").unwrap();
    let podspec = podspec.replace(
        podspec
            .lines()
            .find(|line| line.contains("version"))
            .unwrap(),
        &format!("    s.version = \"{}\"", version),
    );
    std::fs::write("swift/RestsendSdk.podspec", podspec).unwrap();

    let build_dir = tempdir::TempDir::new_in("", ".pods").unwrap();

    let zip_name = build_dir
        .path()
        .join(format!("restsendFFI-{}.xcframework.zip", version))
        .to_str()
        .unwrap()
        .to_string();

    println!("▸ Compress xcframework {}", zip_name);

    // change dir to swift
    std::env::set_current_dir("swift").unwrap();

    let status = Command::new("zip")
        .args(&[
            "-x",
            "*.zip",
            "-r",
            &zip_name,
            "restsendFFI.xcframework",
            "restsend_sdkFFI.h",
            "restsend_sdkFFI.modulemap",
            "restsend_sdk.swift",
        ])
        .status();

    match status {
        Ok(status) => {
            if !status.success() {
                println!("🔥 Compress xcframework failed {:?}", zip_name);
                panic!("zip failed");
            } else {
                println!("🎉 Compress xcframework success {}", zip_name);
            }
        }
        Err(e) => {
            panic!("zip failed: {}", e);
        }
    }
    let zip_size = std::fs::metadata(&zip_name).unwrap().len() as f64 / 1024.0 / 1024.0;
    println!("▸ Compressed xcframework size: {} M", zip_size);

    let status = Command::new("rsync")
        .args(&[&zip_name, &publish_server])
        .status();

    match status {
        Ok(status) => {
            if !status.success() {
                println!("🔥 Upload xcframework failed {:?}", zip_name);
                panic!("rsync failed");
            } else {
                println!("🎉 Upload xcframework success {}", zip_name);
            }
        }
        Err(e) => {
            panic!("rsync failed: {}", e);
        }
    }
}

fn bindgen_with_language(crate_name: &str, args: &Cli) -> Result<String, Error> {
    let ext = std::env::consts::DLL_EXTENSION;

    let language = args.language.clone().unwrap_or("swift".to_string());
    let current_mode = match args.mode.clone() {
        Some(mode) => mode,
        None => {
            let bin_path = std::env::current_exe().unwrap();
            bin_path
                .parent()
                .unwrap()
                .components()
                .last()
                .unwrap()
                .as_os_str()
                .to_str()
                .unwrap()
                .to_string()
        }
    };

    let sources = vec![
        format!(
            "target/aarch64-apple-ios-sim/{}/lib{}.{}",
            current_mode, crate_name, ext
        ),
        format!(
            "target/aarch64-linux-android/{}/lib{}.{}",
            current_mode, crate_name, ext
        ),
        format!("target/{}/lib{}.{}", current_mode, crate_name, ext),
    ];

    let mut mtimes = HashMap::new();
    for source in sources {
        if !PathBuf::from(source.clone()).exists() {
            continue;
        }
        let mtime = std::fs::metadata(&source).unwrap().modified().unwrap();
        mtimes.insert(source.clone(), mtime);
    }

    if mtimes.len() == 0 {
        if std::env::consts::OS == "macos" {
            println!("🔥 No {current_mode},library found run `cargo build --target aarch64-apple-ios-sim --target x86_64-apple-darwin` first");
        } else {
            println!("🔥 No {current_mode} library found run `cargo build` first");
        }
    }

    let mut mtimes: Vec<_> = mtimes.into_iter().collect();
    mtimes.sort_by(|a, b| b.1.cmp(&a.1));
    let source = PathBuf::from(mtimes[0].0.clone());
    println!("▸ Found library {:?}", source);

    let language = PathBuf::from(language.clone());
    let out_dir = Utf8Path::from_path(&language).unwrap();
    let source = Utf8Path::from_path(&source).unwrap();
    let language = language.to_str().unwrap().try_into().unwrap();
    let config_supplier = uniffi_bindgen::EmptyCrateConfigSupplier;

    match language {
        "swift" => {
            uniffi_bindgen::library_mode::generate_bindings(
                source,
                Some(crate_name.to_string()),
                &SwiftBindingGenerator,
                &config_supplier,
                None,
                &out_dir,
                false,
            )
            .expect("generate bindings failed");
        }
        "kotlin" => {
            uniffi_bindgen::library_mode::generate_bindings(
                source,
                Some(crate_name.to_string()),
                &KotlinBindingGenerator,
                &config_supplier,
                None,
                &out_dir,
                false,
            )
            .expect("generate bindings failed");
        }
        "python" => {
            uniffi_bindgen::library_mode::generate_bindings(
                source,
                Some(crate_name.to_string()),
                &PythonBindingGenerator,
                &config_supplier,
                None,
                &out_dir,
                false,
            )
            .expect("generate bindings failed");
        }
        _ => {
            panic!("Unsupported language: {}", language);
        }
    }

    println!(
        r#"🎉 Generate bindings success

    source: {}
    language: {} with {}
    mode: {} "#,
        source, language, crate_name, current_mode
    );
    Ok(current_mode)
}
