extern crate pkg_config;
extern crate vergen;

use vergen::{vergen, SHA};



fn main() {
    pkg_config::probe_library("poppler-glib").unwrap();

    vergen(SHA).unwrap();
}
