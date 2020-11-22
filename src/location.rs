use std::path::PathBuf;

#[derive(Default, Debug)]
pub struct Location {
    file: PathBuf,
    local_line_num: usize,
    global_line_num: usize,
}

impl Location {
    pub fn new(file: impl Into<PathBuf>, local_line_num: usize, global_line_num: usize) -> Self {
        Self {
            file: file.into(),
            local_line_num,
            global_line_num,
        }
    }
}
