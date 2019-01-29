
#[macro_use] extern crate closet;
#[macro_use] extern crate enum_iterator_derive;
#[macro_use] extern crate enum_primitive;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
#[macro_use] extern crate maplit;
#[macro_use] extern crate mrusty;

#[macro_use] mod macro_utils;
#[macro_use] mod logger;
#[macro_use] mod errors;
#[macro_use] mod error_channel;
#[macro_use] mod from_macro;
#[macro_use] mod gtk_utils;
#[macro_use] mod util;

mod app;
mod app_path;
mod archive;
mod cache;
mod chainer;
mod cherenkov;
mod chrysoberyl;
mod clipboard;
mod color;
mod command_line;
mod completion;
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
mod gui;
mod history;
mod image;
mod image_cache;
mod image_fetcher;
mod joiner;
mod key;
mod lazy;
mod lazy_sender;
mod mapping;
mod mruby;
mod operation;
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
mod timer;
mod ui_event;
mod watcher;



fn main() {
    chrysoberyl::main();
}
