use yatt_core::{errors::*, *};

#[macro_use]
extern crate clap;

use std::process::exit;
fn main() {
  if let Err(err) = run(CrateInfo {
    name: crate_name!(),
    version: crate_version!(),
    authors: crate_authors!(),
    description: crate_description!(),
  }) {
    match err {
      CliError::Unexpected { .. } => println!("{}", err),
      CliError::Wrapped { .. } => println!("{}", err),
      _ => {}
    };
    exit(1);
  }
}
