use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
//Simple utilities for splitting a file in multiple chunks of given size.

//Internal struct representing a split request
#[derive(Debug)]
pub struct Settings {
    pub filename: String,
    pub chunk_size: usize, // In bytes!
    pub output_dir: String,
    pub segment_output_filenames: Vec<String>,
}

//Public function, constructs the settings struct and calls the actual splitter
pub fn split(filename: String, chunk_size: usize, output: Option<String>) -> Vec<String> {

    //Decides where the output chunks will end up being
    let output_effective = output.unwrap_or_else(|| String::from("Out/"));

    let mut settings_struct = Settings {
        filename,
        chunk_size,
        output_dir: output_effective.clone(),
        segment_output_filenames: vec![], //Initialized with an empty vector
    };

    // Creates the output dir
    fs::create_dir(&output_effective).unwrap();
    //Actual call to the splitter
    split_file_into_chunks(&mut settings_struct)
}

fn split_file_into_chunks(filespecs: &mut Settings) -> Vec<String> {
    //Open the file
    let mut input_file = OpenOptions::new().read(true).write(false).create(false).open(&filespecs.filename);

    //Initialize the reading buffer with the given size
    let mut buffer = vec![0; filespecs.chunk_size];

    let mut chunk_index = 0;
    loop {
        //File::read only reads until the buffer is full.
        let byte_read = input_file.as_mut().unwrap().read(&mut buffer[..]);

        //If everything has been readed, exit
        match byte_read{
            Ok(0) => break,
            _ => ()
        }
        //Writes the readed chunk to a new file and pushes the path of the file in a vector
        filespecs.segment_output_filenames.push(write_into_chunk(&filespecs.output_dir, chunk_index, &buffer));
        chunk_index += 1;
    }
    return filespecs.segment_output_filenames.clone()
}

fn write_into_chunk(path: &String, index: usize, buffer: &[u8]) -> String {
    let effective_path = format!("{}{}", path, index);

    //Creates the file
    let mut f = File::create(&effective_path).unwrap();
    //Writes all the buffer data to the file
    f.write_all(buffer).unwrap();
    effective_path
}
