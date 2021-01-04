use llvm_profparser::instrumentation_profile::parse;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;

fn main() -> io::Result<()> {
    let path = env::args().nth(1).expect("Expected path to file");

    println!("Going to load: {}", path);
    parse(path)?;
    Ok(())
}
