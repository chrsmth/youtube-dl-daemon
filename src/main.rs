extern crate notify;
extern crate clap;
#[macro_use]
extern crate lazy_static;

use std::ffi::OsStr;
use notify::{Watcher, RecursiveMode, watcher};
use notify::DebouncedEvent;
use std::sync::mpsc::channel;
use std::time::Duration;
use clap::Parser;
use std::process::Command;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

lazy_static! {
    pub static ref YTDL_REGEX: regex::Regex = Regex::new(r".*\.ytdl$").unwrap();
}

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    #[clap(short, long, required=true)]
    path: String,
}

fn process_file(path: PathBuf) {
    let in_file_name = path.file_name().unwrap_or(OsStr::new(""));
    let in_file_name_str = in_file_name.to_string_lossy().into_owned();

    if YTDL_REGEX.is_match(&in_file_name_str) {
        let in_parent = path.parent().unwrap_or(Path::new(""));
        let in_parent_str = in_parent.to_string_lossy().into_owned();
        let in_absolute_path_str = path.as_os_str().to_string_lossy().into_owned();
        let mut run_file_name_str = in_file_name_str.to_owned();
        run_file_name_str.push_str(".running");
        let mut run_absolute_path_pathbuf = path.parent().unwrap_or(Path::new("")).to_owned();
        run_absolute_path_pathbuf.push(Path::new(&run_file_name_str));
        let run_absolute_path_str = run_absolute_path_pathbuf.to_string_lossy().into_owned();

        println!("processing: {:?}", &in_absolute_path_str);
        match fs::rename(&in_absolute_path_str, &run_absolute_path_str) {
            Err(x) => {
                println!("Failed to create run file: {:?}", x);
                return;
            },
            Ok(()) => ()
        }

        let output = Command::new("yt-dlp")
            .args(["-P", &in_parent_str, "--batch", &run_absolute_path_str])
            .output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    println!("ytdl exited with error: {:?}", output);
                    let mut error_file_name = in_file_name_str.to_owned();
                    error_file_name.push_str(".diagnostics");
                    let mut error_absolute_path = path.parent().unwrap_or(Path::new("")).to_owned();
                    error_absolute_path.push(Path::new(&error_file_name));

                    let mut fail_file_name_str = in_file_name_str.to_owned();
                    fail_file_name_str.push_str(".failed");
                    let mut fail_absolute_path_pathbuf = path.parent().unwrap_or(Path::new("")).to_owned();
                    fail_absolute_path_pathbuf.push(Path::new(&fail_file_name_str));
                    let fail_absolute_path_str = fail_absolute_path_pathbuf.to_string_lossy().into_owned();

                    match fs::rename(&run_absolute_path_str, &fail_absolute_path_str) {
                        Err(x) => {
                            println!("Failed to create fail file: {:?}", x);
                            return;
                        },
                        Ok(()) => ()
                    }

                    let mut error_file = File::create(error_absolute_path).unwrap();
                    let _supress_error = writeln!(&mut error_file, "Exited with error code: {:?}\n\n{:?}", output.status, String::from_utf8_lossy(&output.stderr));
                } else {
                    match fs::remove_file(run_absolute_path_str) {
                        Err(x) => {
                            println!("Failed clean run file: {:?}", x);
                        },
                        Ok(()) => ()
                    }
                }
            },
            Err(x) => {
                println!("Failed to execute ytdl: {:?}", x);
            }
        }

        println!("completed: {:?}", &in_absolute_path_str);
    }
}

fn main() {
    let args = Args::parse();
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();
    watcher.watch(args.path, RecursiveMode::Recursive).unwrap();

    loop {
        match rx.recv() {
           Ok(event) => {
               match event {
                   DebouncedEvent::Write(path) | DebouncedEvent::Rename(_, path) | DebouncedEvent::Create(path) => {
                       process_file(path);
                   },
                   _ => (),
               }
           }
           Err(e) => println!("watch error: {:?}", e),
        }
    }
}
