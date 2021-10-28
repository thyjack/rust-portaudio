// The MIT License (MIT)
//
// Copyright (c) 2013 Jeremy Letang (letang.jeremy@gmail.com)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

extern crate pkg_config;

use std::env;
use std::fmt::Display;
use std::path::Path;
use std::process::Command;

#[cfg(all(unix, not(target_os = "linux"), not(target_os = "windows")))]
use unix_platform as platform;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rerun-if-env-changed=PORTAUDIO_ONLY_STATIC");
    if env::var("PORTAUDIO_ONLY_STATIC").is_err() {
        // If pkg-config finds a library on the system, we are done
        if pkg_config::Config::new().atleast_version("19").find("portaudio-2.0").is_ok() {
            return;
        }
    }

    build();
}

fn build() {
    // retrieve cargo deps out dir
    let out_dir_str = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);

    let static_lib = out_dir.join("lib/libportaudio.a");
    if let Err(_) = ::std::fs::metadata(static_lib) {
        platform::download();
        platform::build(out_dir);
    }

    platform::print_libs(out_dir);
}

// Similar to unwrap, but panics on just the error value
#[allow(dead_code)]
fn err_to_panic<T, E: Display>(result: Result<T, E>) -> T {
    match result {
        Ok(x) => x,
        Err(e) => panic!("{}", e)
    }
}

fn run(command: &mut Command) {
    let string = format!("{:?}", command);
    let status = err_to_panic(command.status());
    if !status.success() {
        panic!("`{}` did not execute successfully", string);
    }
}

#[allow(dead_code)]
mod unix_platform {
    use std::process::Command;
    use std::path::Path;

    use std::env;

    use super::{err_to_panic, run};

    pub const PORTAUDIO_URL: &'static str = "http://files.portaudio.com/archives/pa_stable_v190700_20210406.tgz";
    pub const PORTAUDIO_TAR: &'static str = "pa_stable_v190700_20210406.tgz";
    pub const PORTAUDIO_FOLDER: &'static str = "portaudio";

    pub fn download() {
        run(Command::new("curl").arg(PORTAUDIO_URL).arg("-O"));
    }

    pub fn build(out_dir: &Path) {
        // untar portaudio sources
        run(Command::new("tar").arg("xvf").arg(PORTAUDIO_TAR));

        // change dir to the portaudio folder
        err_to_panic(env::set_current_dir(PORTAUDIO_FOLDER));

        // run portaudio autoconf
        run(Command::new("./configure")
            .args(&["--disable-shared", "--enable-static", "--disable-mac-universal"]) // Only build static lib
            .args(&["--prefix", out_dir.to_str().unwrap()]) // Install on the outdir
            .arg("--with-pic")); // Build position-independent code (required by Rust)

        // then make
        run(&mut Command::new("make"));

        // "install" on the outdir
        run(Command::new("make").arg("install"));

        // return to rust-portaudio root
        err_to_panic(env::set_current_dir(".."));

        // cleaning portaudio sources
        run(Command::new("rm").arg("-rf")
            .args(&[PORTAUDIO_TAR, PORTAUDIO_FOLDER]));
    }

    pub fn print_libs(out_dir: &Path) {
        let out_str = out_dir.to_str().unwrap();
        println!("cargo:rustc-flags=-L native={}/lib -l static=portaudio -l framework=CoreServices -l framework=CoreFoundation -l framework=AudioUnit -l framework=AudioToolbox -l framework=CoreAudio", out_str);
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use pkg_config;
    use std::process::Command;
    use super::unix_platform;
    use std::path::Path;

    use super::{run, err_to_panic};

    pub fn download() {
        run(Command::new("wget").arg(unix_platform::PORTAUDIO_URL));
    }

    pub fn build(out_dir: &Path) {
        unix_platform::build(out_dir);
    }

    pub fn print_libs(out_dir: &Path) {
        let portaudio_pc_file = out_dir.join("lib/pkgconfig/portaudio-2.0.pc");
        let portaudio_pc_file = portaudio_pc_file.to_str().unwrap();

        err_to_panic(pkg_config::Config::new().statik(true).find(portaudio_pc_file));
    }
}

#[cfg(windows)]
mod platform {
    use std::path::Path;
    use std::process::Command;
    use std::env;
    use super::{run, err_to_panic};

    #[cfg(target_arch = "x86_64")]
    const PORTAUDIO_DOWNLOAD_URL: &'static str = "https://anaconda.org/anaconda/portaudio/19.6.0/download/win-64/portaudio-19.6.0-he774522_4.tar.bz2";
    #[cfg(target_arch = "x86_64")]
    const PORTAUDIO_TAR: &'static str = "portaudio-19.6.0-he774522_4.tar.bz2";

    const PORTAUDIO_LIB_DIR: &'static str = "portaudio";

    pub fn download() {
        run(Command::new("curl").arg(PORTAUDIO_DOWNLOAD_URL).arg("-O").arg("-s").arg("-L"));
    }

    pub fn build(out_dir: &Path) {
        // move tar to out dir
        let current_dir = err_to_panic(std::env::current_dir());
        let portaudio_location = out_dir.join(PORTAUDIO_TAR);

        err_to_panic(std::fs::rename(
            current_dir.join(PORTAUDIO_TAR), 
            &portaudio_location));

        // change dir to the portaudio folder
        err_to_panic(env::set_current_dir(out_dir));
        let current_dir = out_dir;

        // untar portaudio sources
        run(Command::new("tar").arg("-xjf").arg(current_dir.join(PORTAUDIO_TAR).to_str().unwrap()));

        // move static lib to correct location
        let _ = dbg!(std::fs::create_dir(current_dir.join(PORTAUDIO_LIB_DIR)));
        err_to_panic(std::fs::rename(
            current_dir.join("Library").join("lib").join("portaudio_static.lib"), 
            &current_dir.join(PORTAUDIO_LIB_DIR).join("portaudio.lib")));
    }

    pub fn print_libs(out_dir: &Path) {
        println!("cargo:rustc-link-search={}/{}", out_dir.to_str().unwrap(), PORTAUDIO_LIB_DIR);
    }
}
