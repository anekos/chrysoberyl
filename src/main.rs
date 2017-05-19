
#[macro_use] extern crate closet;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate app_dirs;
extern crate argparse;
extern crate cairo;
extern crate cmdline_parser;
extern crate css_color_parser;
extern crate ctrlc;
extern crate curl;
extern crate encoding;
extern crate env_logger;
extern crate gdk;
extern crate gdk_pixbuf;
extern crate glib;
extern crate gtk;
extern crate immeta;
extern crate libarchive3_sys;
extern crate libarchive;
extern crate libc;
extern crate lru_cache;
extern crate mkstemp;
extern crate num_cpus;
extern crate onig;
extern crate rand;
extern crate shell_escape;
extern crate shellexpand;
extern crate url;

#[macro_use] mod macro_utils;
#[macro_use] mod output;
mod app;
mod app_path;
mod archive;
mod cache;
mod cherenkov;
mod chrysoberyl;
mod color;
mod constant;
mod controller;
mod editor;
mod entry;
mod entry_image;
mod events;
mod filer;
mod fragile_input;
mod gtk_utils;
mod gui;
mod http_cache;
mod image;
mod image_cache;
mod image_fetcher;
mod index_pointer;
mod lazy_sender;
mod mapping;
mod operation;
mod operation_utils;
mod option;
mod option_impls;
mod poppler;
mod script;
mod shell;
mod shellexpand_wrapper;
mod size;
mod sorting_buffer;
mod state;
mod stringer;
mod termination;
mod utils;
mod validation;



fn main() {
    chrysoberyl::main();
}
