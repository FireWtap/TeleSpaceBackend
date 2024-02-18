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
fn merge_chunks_from_list(mut merge_settings: Settings){
    //let mut output = File::create(merge_settings.output_dir.to_string().push_str(&*merge_settings.filename.to_string())).unwrap();\
    fs::create_dir(&merge_settings.output_dir).expect("Can't create output dir");

    let output_path:String = format!("{}{}", merge_settings.output_dir, merge_settings.filename);
    //Creates the output file
    let mut output:File = OpenOptions::new().create(true).write(true).read(true).open(output_path).unwrap();
    //Attaches all the chunks one by one to the end of the newly created file
    for i in merge_settings.segment_input_filenames {
        let mut input = File::open(i).unwrap();
        io::copy(&mut input, &mut output).unwrap();
    }
}