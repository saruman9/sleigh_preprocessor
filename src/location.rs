use std::path::{Path, PathBuf};

#[derive(Default, Debug)]
pub struct Location {
    filepath: PathBuf,
    local_line_num: usize,
    global_line_num: usize,
}

impl Location {
    pub fn new(
        filepath: impl Into<PathBuf>,
        local_line_num: usize,
        global_line_num: usize,
    ) -> Self {
        Self {
            filepath: filepath.into(),
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

    pub fn filepath(&self) -> &Path {
        &self.filepath
    }
}
