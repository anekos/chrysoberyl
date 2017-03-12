
#[macro_use] extern crate closet;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
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
extern crate rand;
extern crate shell_escape;
extern crate url;

#[macro_use] mod output;
#[macro_use] mod utils;
mod app;
mod archive;
mod buffer_cache;
mod chrysoberyl;
mod controller;
mod entry;
mod events;
mod fragile_input;
mod http_cache;
mod index_pointer;
mod key;
mod mapping;
mod operation;
mod options;
mod sorting_buffer;
mod termination;
mod validation;



fn main() {
    chrysoberyl::main();
}
