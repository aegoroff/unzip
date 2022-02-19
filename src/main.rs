use async_std::path::PathBuf;
use clap::{command, Command};
use human_bytes::human_bytes;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::{fs, io};

extern crate async_std;

#[macro_use]
extern crate clap;

#[async_std::main]
async fn main() -> std::io::Result<()> {
    let app = build_cli();
    let matches = app.get_matches();

    let zip = matches.value_of("zip").unwrap();
    let extract = matches.value_of("extract").unwrap();

    let fname = std::path::Path::new(zip);

    #[cfg(target_os = "windows")]
    let dir = String::from(r"\\?\") + extract;
    #[cfg(target_os = "windows")]
    let extract_to = std::path::Path::new(&dir);

    #[cfg(not(target_os = "windows"))]
    let extract_to = std::path::Path::new(extract);

    let file = fs::File::open(&fname)?;

    let mut archive = zip::ZipArchive::new(file)?;

    let mut total = 0u64;

    let bar = ProgressBar::new(archive.len() as u64);
    bar.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {percent}% ({per_sec}, {eta})\n  {wide_msg}")
        .progress_chars("=>-"));

    for i in 0..archive.len() {
        // each 256 item
        if i & (256 - 1) == 0 {
            bar.set_position(i as u64);
        }

        let mut zip_file = archive.by_index(i)?;
        let outpath = match zip_file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        let result_path = make_result_path(extract_to, &outpath);

        if (&*zip_file.name()).ends_with('/') {
            create_directory(&result_path).await;
        } else {
            if let Some(parent) = result_path.parent() {
                if !parent.exists().await {
                    create_directory(parent).await
                }
            }
            let outfile = fs::File::create(&result_path);
            match outfile {
                Ok(mut p) => {
                    let r = io::copy(&mut zip_file, &mut p);
                    if let Ok(r) = r {
                        total += r;
                    }
                }
                Err(e) => {
                    println!(
                        "Error: {} extracting path: {}",
                        e,
                        outpath.to_str().unwrap_or_default()
                    );
                }
            };
        }
    }
    bar.set_position(archive.len() as u64);
    bar.finish_with_message(format!("Extracted: {}", human_bytes(total as f64)));
    Ok(())
}

async fn create_directory(path: &async_std::path::Path) {
    match async_std::fs::create_dir_all(path).await {
        Ok(_) => {}
        Err(e) => {
            println!(
                "Error: {} path to create: {}",
                e,
                path.to_str().unwrap_or_default()
            );
        }
    }
}

#[cfg(target_os = "windows")]
#[inline]
fn make_result_path(directory: &Path, zip_path: &Path) -> PathBuf {
    zip_path
        .to_str()
        .unwrap_or_default()
        .split('/')
        .fold(PathBuf::from(directory), |pb, s| pb.join(s))
}

#[cfg(not(target_os = "windows"))]
#[inline]
fn make_result_path(directory: &Path, zip_path: &Path) -> PathBuf {
    PathBuf::from(directory.join(zip_path))
}

fn build_cli() -> Command<'static> {
    return command!(crate_name!())
        .arg_required_else_help(true)
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .arg(arg!([zip]).help("Path to zip file").required(true).index(1))
        .arg(
            arg!(-e --extract <PATH>)
                .required(true)
                .takes_value(true)
                .help("Output directory path"),
        );
}
