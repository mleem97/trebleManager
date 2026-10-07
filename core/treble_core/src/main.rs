//! `ttcore` — thin CLI over treble_core for future PS1/bash wrappers.
//!
//! Plain-text output, exit codes carry the verdict:
//! `image-kind` → boot|system|unknown (0/2); `flash-verdict` (stdin) → 0/1/2;
//! `check-url` → host or ERROR (0/3).

use std::io::Read;
use std::path::PathBuf;
use treble_core::{fastboot, firmware, images};

fn main() {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_default();
    let code = match cmd.as_str() {
        "image-kind" => {
            let p: PathBuf = args.next().unwrap_or_default().into();
            match images::image_kind(&p) {
                images::ImageKind::Boot => {
                    println!("boot");
                    0
                }
                images::ImageKind::System => {
                    println!("system");
                    0
                }
                images::ImageKind::Unknown => {
                    println!("unknown");
                    2
                }
            }
        }
        "flash-verdict" => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).unwrap_or_default();
            let lines: Vec<&str> = input.lines().collect();
            match fastboot::flash_verdict(lines) {
                fastboot::Verdict::Ok { okay, total_time } => {
                    println!("OK ({okay} OKAY, {total_time})");
                    0
                }
                fastboot::Verdict::Failed { lines } => {
                    println!("FAILED");
                    for l in lines.iter().take(3) {
                        println!(" ! {l}");
                    }
                    1
                }
                fastboot::Verdict::Unclear => {
                    println!("UNCLEAR");
                    2
                }
            }
        }
        "check-url" => {
            let u = args.next().unwrap_or_default();
            match firmware::check_url(&u) {
                Ok(c) => {
                    println!("OK {}", c.host);
                    0
                }
                Err(e) => {
                    println!("ERROR {e:?}");
                    3
                }
            }
        }
        _ => {
            eprintln!("ttcore: image-kind <file> | flash-verdict | check-url <url>");
            4
        }
    };
    std::process::exit(code);
}
