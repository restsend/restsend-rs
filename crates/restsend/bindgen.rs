use std::collections::HashMap;
use std::io::Error;
use std::path::PathBuf;
use std::process::Command;
use std::vec;

use camino::Utf8Path;
use clap::{Parser, ValueEnum};

use uniffi_bindgen::bindings::TargetLanguage;
use uniffi_bindgen::{self, BindingGeneratorDefault};

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
}
fn main() {
    let args = Cli::parse();
    let publish = args.publish.unwrap_or(false);

    let crate_name = std::env::var("CARGO_PKG_NAME")
        .unwrap_or("restsend_ffi".to_string())
        .replace("-", "_");
    let crate_name = crate_name.as_str();

    let (lang, mode) = bindgen_with_language(crate_name, args).unwrap();

    match lang {
        TargetLanguage::Swift => {
            // only mac support xcframework
            if std::env::consts::OS != "macos" {
                println!("Only macos support xcframework build");
                return;
            }
            build_xcframework(crate_name, mode.clone());
            if publish {
                if mode == "release" {
                    publish_ios_pods();
                } else {
                    println!("🔥 Only publish release mode");
                }
            }
        }
        _ => return,
    }
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

fn publish_ios_pods() {
    // Get version from `swift/RestsendSdk.podspec`
    let podspec = std::fs::read_to_string("swift/RestsendSdk.podspec").unwrap();
    let version = podspec
        .lines()
        .find(|line| line.contains("s.version"))
        .unwrap()
        .split("\"")
        .nth(1)
        .unwrap();

    println!("Publish version: {}", version);

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
        .args(&["-x", "*.zip", "-r", &zip_name, "restsendFFI.xcframework"])
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

    let status = Command::new("scp")
        .args(&[&zip_name, "ubuntu@chat.ruzhila.cn:/var/www/chat/downloads/"])
        .status();

    match status {
        Ok(status) => {
            if !status.success() {
                println!("🔥 Upload xcframework failed {:?}", zip_name);
                panic!("scp failed");
            } else {
                println!("🎉 Upload xcframework success {}", zip_name);
            }
        }
        Err(e) => {
            panic!("scp failed: {}", e);
        }
    }
}

fn bindgen_with_language(crate_name: &str, args: Cli) -> Result<(TargetLanguage, String), Error> {
    let ext = std::env::consts::DLL_EXTENSION;

    let language = args.language.unwrap_or("swift".to_string());
    let current_mode = match args.mode {
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
            println!("🔥 No library found run `cargo build --target aarch64-apple-ios-sim --target x86_64-apple-darwin` first");
        } else {
            println!("🔥 No library found run `cargo build` first");
        }
    }

    let mut mtimes: Vec<_> = mtimes.into_iter().collect();
    mtimes.sort_by(|a, b| b.1.cmp(&a.1));
    let source = PathBuf::from(mtimes[0].0.clone());
    println!("▸ Found library {:?}", source);

    let language = PathBuf::from(language.clone());
    let out_dir = Utf8Path::from_path(&language).unwrap();
    let source = Utf8Path::from_path(&source).unwrap();
    let language = TargetLanguage::from_str(language.to_str().unwrap(), true).unwrap();

    uniffi_bindgen::library_mode::generate_bindings(
        source,
        Some(crate_name.to_string()),
        &BindingGeneratorDefault {
            target_languages: vec![language],
            try_format_code: false,
        },
        None,
        &out_dir,
        false,
    )
    .unwrap();

    println!(
        r#"🎉 Generate bindings success

    source: {}
    language: {} with {}
    mode: {} "#,
        source, language, crate_name, current_mode
    );
    Ok((language, current_mode))
}
