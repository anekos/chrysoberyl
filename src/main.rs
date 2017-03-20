
#[macro_use] extern crate closet;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate app_dirs;
extern crate argparse;
extern crate cairo;
extern crate cmdline_parser;
extern crate ctrlc;
extern crate encoding;
extern crate env_logger;
extern crate gdk;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate hyper;
extern crate hyper_native_tls;
extern crate immeta;
extern crate libarchive3_sys;
extern crate libarchive;
extern crate libc;
extern crate onig;
extern crate rand;
extern crate shell_escape;
extern crate shellexpand;
extern crate url;

#[macro_use] mod output;
#[macro_use] mod utils;
mod app;
mod app_path;
mod archive;
mod chrysoberyl;
mod command;
mod config;
mod controller;
mod entry;
mod events;
mod fragile_input;
mod http_cache;
mod index_pointer;
mod mapping;
mod operation;
mod operation_utils;
mod options;
mod shell;
mod sorting_buffer;
mod termination;
mod validation;



fn main() {
    chrysoberyl::main();
}
