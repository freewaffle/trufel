#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::fs::{File, write};
use std::path::Path;
use std::io::{Read, Seek};

mod magic;
mod vm;
mod instr;
mod asm;
// mod compiler;

use magic::*;

macro_rules! error_exit {
    () => {
        std::process::exit(1)
    };
}

#[inline]
fn usage() {
    println!("usage: trufel [COMMAND] [FILENAME]");
    println!();
    println!("FILENAME is a Trufcode source or binary file");
    println!();
    println!("commands:");
    println!("    run\truns file (compiling first if it's");
    println!("       \ta source file)");
    println!("    compile\tcompiles file and produces .truf file");
    println!("    help\tprints this text");
    println!("    version\tshow version and license information");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let command = if let Some(cmd) = args.get(1) {
        cmd.as_str()
    } else {
        eprintln!("error: missing command");
        eprintln!("try executing: trufel help");
        error_exit!();
    };

    match command {
        "run" | "compile" => {
            let compile_only = command == "compile";

            let filename = if let Some(path) = args.get(2) {
                path
            } else {
                eprintln!("error: missing filename");
                eprintln!("try executing: trufel help");
                error_exit!();
            };

            let path = Path::new(filename);

            let stem = if path.is_file() {
                path.file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
            } else {
                eprintln!("error: {filename} is not a file");
                error_exit!();
            };

            let mut file = match File::open(filename) {
                Ok(file) => file,
                Err(err) => {
                    eprintln!("error: can't open {filename}: {}", err.kind());
                    error_exit!();
                }
            };

            let mut this_magic = [0u8; MAGIC_LEN];

            let bytes_read = match file.read(&mut this_magic) {
                Ok(bytes) => bytes,
                Err(err) => {
                    eprintln!("error: can't read {filename}: {}", err.kind());
                    error_exit!();
                }
            };

            if bytes_read < MAGIC_LEN {
                eprintln!("error: {filename} is too short");
                error_exit!();
            }

            let is_binary = this_magic == MAGIC;
            let mut code: Vec<u8>;
            
            if is_binary {
                if compile_only {
                    eprintln!("error: {filename} is a Trufcode binary");
                    error_exit!();
                }

                code = Vec::new();
                file.rewind().unwrap();
                if let Err(err) = file.read_to_end(&mut code) {
                    eprintln!("error: can't read {filename}: {}", err.kind());
                    error_exit!();
                }
            } else {
                code = if let Ok(code) = asm::compile_from_file(file, filename.clone()) {
                    code
                } else {
                    // error details should have been printed
                    error_exit!();
                };
            }

            if compile_only {
                let filename = format!("{stem}.truf");
                if let Err(err) = write(&filename, code) {
                    eprintln!("error: failed to write to {filename}: {}", err.kind());
                    error_exit!();
                }
            } else {
                // replace this with `start_vm(code)`-like function
                drop(code);
            }
        }

        "help" => usage(),

        "version" => {
            println!("Trufel v{} <https://github.com/freewaffle/trufel>", env!("CARGO_PKG_VERSION"));
            println!("Copyright (C) 2026 freewaffle");
            println!();
            println!("this program comes with ABSOLUTELY NO WARRANTY.");
            println!("this is free software, and you are welcome to redistribute it.");
        }

        _ => {
            eprintln!("error: invalid command");
            eprintln!("try executing: trufel help");
            error_exit!();
        }
    }
}

/* use raylib::prelude::*;

fn main() {
    let (mut rl, rl_thread) = raylib::init()
        .size(640, 480)
        .title("Trufel")
        .build();

    while !rl.window_should_close() {
        rl.draw(&rl_thread, | mut drawer | {
            drawer.clear_background(Color::WHITE);
            drawer.draw_text("Hello, world!", 12, 12, 20, Color::BLACK);
        });
    }
} */
