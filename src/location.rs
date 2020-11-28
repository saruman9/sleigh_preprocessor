use std::path::{Path, PathBuf};

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

    pub fn global_line_num(&self) -> usize {
        self.global_line_num
    }

    pub fn local_line_num(&self) -> usize {
        self.local_line_num
    }

    pub fn path(&self) -> &Path {
        &self.file
    }
}
