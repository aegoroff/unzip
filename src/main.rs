use async_std::path::PathBuf;
use async_std::task;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::{fs, io};

extern crate async_std;

fn main() {
    task::block_on(async_main())
}

async fn async_main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <filename> <dirname>", args[0]);
        return;
    }
    let fname = std::path::Path::new(&*args[1]);

    #[cfg(target_os = "windows")]
    let dir = String::from(r"\\?\") + &*args[2];
    #[cfg(target_os = "windows")]
    let extract_to = std::path::Path::new(&dir);

    #[cfg(not(target_os = "windows"))]
    let extract_to = std::path::Path::new(&*args[2]);

    let file = fs::File::open(&fname).unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();

    let mut total = 0u64;

    let bar = ProgressBar::new(archive.len() as u64);
    bar.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {percent}% ({per_sec}, {eta})")
        .progress_chars("#>-"));

    for i in 0..archive.len() {
        if i & (256 - 1) == 0 {
            bar.set_position(i as u64);
        }

        let mut zip_file = archive.by_index(i).unwrap();
        let outpath = match zip_file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        let result_path = make_result_path(extract_to, &outpath);

        if (&*zip_file.name()).ends_with('/') {
            async_std::fs::create_dir_all(&result_path).await.unwrap();
        } else {
            if let Some(p) = result_path.parent() {
                if !p.exists().await {
                    let r = async_std::fs::create_dir_all(&p).await;
                    match r {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!(
                                "Error: {} path to create: {}",
                                e,
                                p.to_str().unwrap_or_default()
                            );
                        }
                    }
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
                    eprintln!(
                        "Error: {} extracting path: {}",
                        e,
                        outpath.to_str().unwrap_or_default()
                    );
                }
            };
        }
    }
    println!("Extracted {} bytes", total);
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
