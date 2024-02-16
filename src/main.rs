use std::{env, io};
use std::fs::File;

mod splitting_utils;

fn main() -> io::Result<()> {


    let parts = splitting_utils::file_splitter::split(String::from("prove.mp4"), 1024 * 1024 * 10, None);

    splitting_utils::file_merger::merge(String::from("prove.mp4"), String::from("output/"),parts);

/*

    eprintln!("done");*/
    Ok(())
}
