use std::{
    path::{PathBuf, Path},
    fs,
    process::Command,
    io::Write,
};

use clap::Parser;
use chrono::Utc;
use zip::{ZipWriter, write::SimpleFileOptions};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "ADDIN_PATH")]
    addin: String,
    #[arg(short, long, value_name = "OUT")]
    out: String,
    #[arg(short, long, default_value_t = false)]
    release: bool,
}

struct BuildTarget {
    cmd: &'static str,
    triple: &'static str,
    arch: &'static str,
    os: &'static str,
    archos: &'static str,
    ext: &'static str,
}

const PKG_NAME: &'static str = "common_addin";
const PKG_NAME_LINUX: &'static str = "libcommon_addin";

const BUILD_TARGETS: [BuildTarget;4] = [
    BuildTarget { cmd: "cargo", triple: "i686-pc-windows-msvc",     arch: "i386",   os: "Windows", archos: "win32", ext: "dll" },
    BuildTarget { cmd: "cargo", triple: "x86_64-pc-windows-msvc",   arch: "x86_64", os: "Windows", archos: "win64", ext: "dll" },
    BuildTarget { cmd: "cross", triple: "i686-unknown-linux-gnu",   arch: "i386",   os: "Linux", archos: "linux32", ext: "so" },
    BuildTarget { cmd: "cross", triple: "x86_64-unknown-linux-gnu", arch: "x86_64", os: "Linux", archos: "linux64", ext: "so" },
];

fn main() {
    let cli = Cli::parse();

    let release_or_debug = if cli.release { "release" } else { "debug" };
    let addin_manifest_path = PathBuf::from(&cli.addin)
        .join("Cargo.toml");

    //

    let now = Utc::now();
    let timestamp_str3389 = now.to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    let timestamp_str = format!("{}", now.format("%Y%m%d%H%M%S"));

    let timestamp_txt_path = Path::new(&cli.addin).join("compilation_timestamp.txt");
    fs::write(&timestamp_txt_path, timestamp_str3389.as_bytes()).unwrap();

    //

    let mut manifest = String::new();
    manifest.push_str( "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n" );
    manifest.push_str( "<bundle xmlns=\"http://v8.1c.ru/8.2/addin/bundle\" name=\"CommonAddin\">\n" );

    //

    let file = fs::File::create(&cli.out).unwrap();

    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    for build_target in BUILD_TARGETS {
        let mut cmd = Command::new(build_target.cmd);
        cmd.arg("build");
        cmd.arg("--manifest-path");
        cmd.arg(&addin_manifest_path);
        cmd.arg("--target");
        cmd.arg(build_target.triple);
        
        if cli.release {
            cmd.arg("--release");
        }

        let status = cmd
            .spawn()
            .unwrap()
            .wait_with_output()
            .unwrap();
        assert!( status.status.success() );

        let pkg_name = if build_target.os == "Linux" { PKG_NAME_LINUX } else { PKG_NAME };

        let original_dll_name = String::new() + 
            pkg_name +
            "." + 
            build_target.ext;

        let dll_with_timestamp_name = String::new() + 
            pkg_name +
            "." +
            build_target.archos +
            "." +
            &timestamp_str +
            "." + 
            build_target.ext;
        
        let original_dll_path = PathBuf::from(&cli.addin)
            .join("target")
            .join(build_target.triple)
            .join(release_or_debug)
            .join(&original_dll_name);

        assert!( &original_dll_path.exists() );
        let content = fs::read(original_dll_path).unwrap();

        zip.start_file(&dll_with_timestamp_name, options).unwrap();
        zip.write_all(&content).unwrap();

        let manifest_line = String::new() + 
            "	<component os=\"" + 
            build_target.os + 
            "\" path=\"" +
            &dll_with_timestamp_name + 
            "\" type=\"native\" arch=\"" +
            build_target.arch + 
            "\" />\n";      
        
        manifest.push_str(&manifest_line);
    }

    fs::remove_file( &timestamp_txt_path ).unwrap();

    manifest.push_str( "</bundle>\n" );

    zip.start_file("manifest.xml", options).unwrap();
    zip.write_all(manifest.as_bytes()).unwrap();

    zip.finish().unwrap();
}
