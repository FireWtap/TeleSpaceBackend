use std::fs::{File, OpenOptions};
use std::{fs, io};

pub struct Settings {
    pub filename: String,
    pub segment_input_filenames: Vec<String>,
    pub output_dir: String
}

pub fn merge(filename: String, output_dir: String, segment_input_filenames: Vec<String>) -> (){
    let merge_settings = Settings{
        filename, output_dir, segment_input_filenames
    };
    merge_chunks_from_list(merge_settings)

}
pub fn merge_chunks_from_list(mut merge_settings: Settings){
    //let mut output = File::create(merge_settings.output_dir.to_string().push_str(&*merge_settings.filename.to_string())).unwrap();\
    fs::create_dir(&merge_settings.output_dir).unwrap();

    let output_path = format!("{}{}", merge_settings.output_dir, merge_settings.filename);
    println!("{}", output_path);
    let mut output = OpenOptions::new().create(true).write(true).read(true).open(output_path).unwrap();
    for i in merge_settings.segment_input_filenames {
        let mut input = File::open(i).unwrap();
        io::copy(&mut input, &mut output).unwrap();
    }
}