#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::fs::File;
use std::path::Path;
use std::io::{Read, Seek};

mod magic;
mod vm;
mod assembler;
// mod compiler;

use magic::*;

macro_rules! error_exit {
    () => {
        std::process::exit(1)
    };
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let filename = if let Some(path) = args.get(1) {
        path.to_owned()
    } else {
        eprintln!("error: missing file name");
        eprintln!("try executing: trufel [FILENAME]");
        error_exit!();
    };

    let path = Path::new(&filename);

    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("error: can't open {filename}: {}", err.kind());
            error_exit!();
        }
    };

    let mut this_magic = [0u8; MAGIC_LEN];

    {
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
    }

    let code: Vec<u8> = if this_magic == MAGIC {
        let mut code: Vec<u8> = Vec::new();
        if let Err(err) = file.read_to_end(&mut code) {
            eprintln!("error: can't read {filename}: {}", err.kind());
            error_exit!();
        } else {
            code
        }
    } else {
        println!("compiling...");

        let extension = if let Some(ext) = path.extension() {
            ext.to_str().unwrap()
        } else {
            eprintln!("error: cannot determine what kind of source code is file {filename}");
            eprintln!("this file must have one of these extensions:");
            eprintln!(" - .trs for assembly code");
            eprintln!(" - .trl for Trulang code");
            error_exit!();
        };

        file.rewind().unwrap();

        match extension {
            "trs" => {
                #[allow(irrefutable_let_patterns)]
                if let Ok(code) = assembler::compile_from_file(file, filename) {
                    code
                } else {
                    // error details should have been printed
                    error_exit!();
                }
            }

            "trl" => {
                eprintln!("error: .trl files are not currently supported, as the Trulang compiler is unfinished");
                error_exit!();

                /* if let Ok(code) = compiler::compile_from_file(file, path) {
                    code
                } else {
                    eprintln!("error: can't compile");
                    error_exit!();
                } */
            }

            _ => {
                eprintln!("error: unrecognized filename extension");
                error_exit!();
            }
        }
    };

    // replace this with `start_vm(code)`-like function
    drop(code);
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
