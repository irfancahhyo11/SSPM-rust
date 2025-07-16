use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

const DEFAULT_REPO: &str = "https://raw.githubusercontent.com/irfancahhyo11/packages/main/";
const REPO_FILE: &str = "repo.rlpmrepo";
const SSPM_DIR: &str = "C:/rlpm/sspm";

fn get_repo_url() -> String {
    fs::read_to_string(REPO_FILE).unwrap_or_else(|_| DEFAULT_REPO.to_string()).trim().to_string()
}

fn check_sspm_folder() {
    if !Path::new(SSPM_DIR).exists() {
        fs::create_dir_all(SSPM_DIR).expect("Failed to create sspm dir");
    }
}

fn download_file(url: &str, dest: &str) -> Result<(), reqwest::Error> {
    let mut resp = reqwest::blocking::get(url)?;
    let mut out = fs::File::create(dest).expect("Failed to create file");
    io::copy(&mut resp, &mut out).expect("Failed to write file");
    Ok(())
}

fn install_package(package: &str, action: &str) {
    let sspm_path = format!("{}/{}.sspm", SSPM_DIR, package);
    let rlpm_path = format!("{}/{}.rlpm", SSPM_DIR, package);
    let script_path = if Path::new(&sspm_path).exists() {
        sspm_path
    } else if Path::new(&rlpm_path).exists() {
        rlpm_path
    } else {
        println!("No .sspm or .rlpm file found for {}", package);
        return;
    };
    // Parse script file for 'source' and 'format'
    let content = fs::read_to_string(&script_path).expect("Failed to read script");
    let mut source = String::new();
    let mut format = String::new();
    for line in content.lines() {
        if line.starts_with("source=") {
            source = line[7..].trim().to_string();
        } else if line.starts_with("format=") {
            format = line[7..].trim().to_string();
        }
    }
    if source.is_empty() || format.is_empty() {
        println!("Invalid package script: missing source or format");
        return;
    }
    let pkg_dir = format!("{}/{}-src", SSPM_DIR, package);
    fs::create_dir_all(&pkg_dir).expect("Failed to create src dir");
    let filename = source.split('/').last().unwrap_or("downloaded");
    let archive_path = format!("{}/{}", pkg_dir, filename);
    println!("Downloading {}...", source);
    if let Err(e) = download_file(&source, &archive_path) {
        println!("Download failed: {}", e);
        return;
    }
    // Extract
    if format == "tar" {
        let file = fs::File::open(&archive_path).expect("Failed to open archive");
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
        archive.unpack(&pkg_dir).expect("Failed to unpack tar");
    } else if format == "zip" {
        let file = fs::File::open(&archive_path).expect("Failed to open archive");
        let mut archive = zip::ZipArchive::new(file).expect("Failed to open zip");
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = Path::new(&pkg_dir).join(file.name());
            if file.is_dir() {
                fs::create_dir_all(&outpath).unwrap();
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p).unwrap();
                    }
                }
                let mut outfile = fs::File::create(&outpath).unwrap();
                io::copy(&mut file, &mut outfile).unwrap();
            }
        }
    }
    // Run install/remove if needed (not implemented: run script)
    println!("{} {} complete!", action, package);
    // Clean up
    fs::remove_file(&archive_path).ok();
    fs::remove_dir_all(&pkg_dir).ok();
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        println!("Usage: rlpm [install|remove] <package>");
        return;
    }
    let repo_url = get_repo_url();
    let mut action = "NONE";
    let mut packages = vec![];
    for arg in &args {
        if arg == "install" {
            action = "INSTALL";
        } else if arg == "remove" {
            action = "REMOVE";
        } else {
            packages.push(arg.clone());
        }
    }
    check_sspm_folder();
    for package in packages {
        let sspm = format!("{}.sspm", package);
        let rlpm = format!("{}.rlpm", package);
        let local_sspm = Path::new(&sspm);
        let local_rlpm = Path::new(&rlpm);
        if !local_sspm.exists() && !local_rlpm.exists() {
            println!("Installing {} from repo", package);
            let url = format!("{}/{}.sspm", repo_url.trim_end_matches('/'), package);
            let dest = format!("{}/{}.sspm", SSPM_DIR, package);
            if download_file(&url, &dest).is_err() {
                // Try .rlpm
                let url2 = format!("{}/{}.rlpm", repo_url.trim_end_matches('/'), package);
                let dest2 = format!("{}/{}.rlpm", SSPM_DIR, package);
                if download_file(&url2, &dest2).is_err() {
                    println!("Failed to download {}.sspm or .rlpm", package);
                    continue;
                }
            }
            install_package(&package, action);
        } else {
            println!("Installing local {}", package);
            let src = if local_sspm.exists() { sspm } else { rlpm };
            let dest = format!("{}/{}", SSPM_DIR, &src);
            fs::copy(&src, &dest).expect("Failed to copy local script");
            install_package(&package, action);
        }
    }
}
