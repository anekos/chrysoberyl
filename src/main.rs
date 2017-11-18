
#[macro_use] extern crate closet;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
#[macro_use] extern crate maplit;
#[macro_use] extern crate mrusty;
extern crate app_dirs;
extern crate argparse;
extern crate atty;
extern crate cairo;
extern crate cmdline_parser;
extern crate css_color_parser;
extern crate ctrlc;
extern crate curl;
extern crate encoding;
extern crate env_logger;
extern crate filetime;
extern crate gdk;
extern crate gdk_pixbuf;
extern crate glib;
extern crate globset;
extern crate gtk;
extern crate immeta;
extern crate libarchive3_sys;
extern crate libarchive;
extern crate libc;
extern crate lru_cache;
extern crate md5;
extern crate mkstemp;
extern crate natord;
extern crate num_cpus;
extern crate pom;
extern crate rand;
extern crate readline;
extern crate shell_escape;
extern crate shellexpand;
extern crate time;
extern crate tldextract;
extern crate url;

#[macro_use] mod macro_utils;
#[macro_use] mod logger;
#[macro_use] mod error;
#[macro_use] mod errors;
#[macro_use] mod from_macro;

mod app;
mod app_path;
mod archive;
mod cache;
mod cherenkov;
mod chrysoberyl;
mod color;
mod command_line;
mod completer;
mod config;
mod constant;
mod controller;
mod counter;
mod editor;
mod entry;
mod events;
mod expandable;
mod file_extension;
mod filer;
mod filterable_vec;
mod fragile_input;
mod gtk_utils;
mod gui;
mod image;
mod image_cache;
mod image_fetcher;
mod key;
mod lazy;
mod lazy_sender;
mod mapping;
mod mruby;
mod operation;
mod operation_utils;
mod option;
mod paginator;
mod poppler;
mod remote_cache;
mod resolution;
mod script;
mod session;
mod shell;
mod shell_filter;
mod shellexpand_wrapper;
mod shorter;
mod size;
mod sorting_buffer;
mod state;
mod termination;
mod timer;
mod ui_event;
mod util;
mod version;



fn main() {
    use std::env::args;

    if args().nth(1).as_ref().map(String::as_str) == Some("--complete") {
        completer::main();
    } else {
        chrysoberyl::main();
    }
}
