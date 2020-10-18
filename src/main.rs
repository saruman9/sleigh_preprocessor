use std::collections::HashMap;
use std::env::args;
use std::fs::File;
use std::io::Write;

use sleigh_preprocessor::SleighPreprocessor;

fn main() {
    pretty_env_logger::init();
    let mut writer = String::new();
    let mut definitions = HashMap::new();
    let file_path = args().nth(1).unwrap();
    let mut sleigh_preprocessor = SleighPreprocessor::new(definitions, &file_path);
    definitions = sleigh_preprocessor.process(&mut writer).unwrap();
    let mut new_file =
        File::create(std::path::PathBuf::from(&file_path).with_extension("sla")).unwrap();
    println!("{:?}", definitions);
    new_file.write_all(&writer.into_bytes()).unwrap();
}
