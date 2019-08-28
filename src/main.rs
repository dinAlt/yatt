#[macro_use]
extern crate clap;

use std::process::exit;

fn main() {
    if cli::run(cli::CrateInfo {
        name: crate_name!(),
        version: crate_version!(),
        authors: crate_authors!(),
        description: crate_description!(),
    })
    .is_err()
    {
        exit(1);
    }
}
