use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

#[derive(Debug)]
pub struct Settings {
    pub filename: String,
    pub chunk_size: usize, // In bytes!
    pub output_dir: String,
    pub segment_output_filenames: Vec<String>,
}

pub fn split(filename: String, chunk_size: usize, output: Option<String>) -> Vec<String> {
    let output_effective = output.unwrap_or_else(|| filename.clone() + "Out/");

    let mut settings_struct = Settings {
        filename,
        chunk_size,
        output_dir: output_effective.clone(),
        segment_output_filenames: vec![],
    };

    // Crea la cartella di output
    fs::create_dir(&output_effective).unwrap();

    split_file_into_chunks(&mut settings_struct)
}

fn split_file_into_chunks(filespecs: &mut Settings) -> Vec<String> {
    let mut input_file = OpenOptions::new().read(true).write(false).create(false).open(&filespecs.filename);

    let mut buffer = vec![0; filespecs.chunk_size];
    let mut chunk_index = 0;

    loop {
        let byte_read = input_file.as_mut().unwrap().read(&mut buffer[..]);
        match byte_read{
            Ok(0) => break,
            _ => ()
        }

        filespecs.segment_output_filenames.push(write_into_chunk(&filespecs.filename, &filespecs.output_dir, chunk_index, &buffer));
        chunk_index += 1;
    }
    return filespecs.segment_output_filenames.clone()
}

fn write_into_chunk(filename: &String, path: &String, index: usize, buffer: &[u8]) -> String {
    let effective_path = format!("{}{}", path, index);
    let mut f = File::create(&effective_path).unwrap();

    f.write_all(buffer).unwrap();
    return effective_path
}
