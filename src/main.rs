use std::collections::HashMap;
use std::env::args;
use std::fs::File;
use std::io::Write;

use sleigh_preprocessor::SleighPreprocessor;

fn main() {
    pretty_env_logger::init();
    let mut writer = String::new();
    let definitions = HashMap::new();
    let file_path = args().nth(1).unwrap();
    let mut sleigh_preprocessor = SleighPreprocessor::new(definitions, &file_path, true);
    if let Err(e) = sleigh_preprocessor.process(&mut writer) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
    println!("{:#?}", sleigh_preprocessor.definitions());
    println!("{:#?}", sleigh_preprocessor.locations());
    let mut new_file =
        File::create(std::path::PathBuf::from(&file_path).with_extension("sla")).unwrap();
    new_file.write_all(&writer.into_bytes()).unwrap();
}
