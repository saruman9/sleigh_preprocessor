use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use log::trace;
use regex::Regex;

mod conditional_helper;

use conditional_helper::ConditionalHelper;

type Definitions = HashMap<String, String>;

lazy_static::lazy_static! {
    static ref EXPANSION_RE: Regex = Regex::new(r"\$\(([0-9A-Z_a-z]+)\)").unwrap();
    static ref INCLUDE_RE: Regex = Regex::new(r#"^\s*@include\s+"(.*)"\s*$"#).unwrap();
    static ref DEFINE1_RE: Regex = Regex::new(r#"^\s*@define\s+([0-9A-Z_a-z]+)\s+"(.*)"\s*$"#).unwrap();
    static ref DEFINE2_RE: Regex = Regex::new(r"^\s*@define\s+([0-9A-Z_a-z]+)\s+(\S+)\s*$").unwrap();
    static ref DEFINE3_RE: Regex = Regex::new(r"^\s*@define\s+([0-9A-Z_a-z]+)\s*$").unwrap();
    static ref UNDEF_RE: Regex = Regex::new(r"^\s*@undef\s+([0-9A-Z_a-z]+)\s*$").unwrap();
    static ref IFDEF_RE: Regex = Regex::new(r"^\s*@ifdef\s+([0-9A-Z_a-z]+)\s*$").unwrap();
    static ref IFNDEF_RE: Regex = Regex::new(r"^\s*@ifndef\s+([0-9A-Z_a-z]+)\s*$").unwrap();
    static ref IF_RE: Regex = Regex::new(r"^\s*@if\s+(.*)").unwrap();
    static ref ELIF_RE: Regex = Regex::new(r"^\s*@elif\s+(.*)").unwrap();
    static ref ENDIF_RE: Regex = Regex::new(r"^\s*@endif\s*$").unwrap();
    static ref ELSE_RE: Regex = Regex::new(r"^\s*@else\s*$").unwrap();
    static ref FULL_LINE_COMMENT_RE: Regex = Regex::new(r"^\s*#.*").unwrap();
    static ref COMMENT_RE: Regex = Regex::new(r"#.*").unwrap();
}

#[derive(Debug, Default)]
pub struct SleighPreprocessor<'a> {
    definitions: Option<Definitions>,
    compatible: bool,

    ifstack: Vec<ConditionalHelper>,
    error_count: u64,

    file_path: PathBuf,
    line: Option<&'a str>,
    line_no: u64,
    overall_line_no: u64,
}

impl<'a> SleighPreprocessor<'a> {
    pub fn new<P>(definitions: Definitions, file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        SleighPreprocessor {
            definitions: Some(definitions),
            file_path: file_path.into(),
            ..Default::default()
        }
    }

    fn include_file<P>(
        &mut self,
        writer: &mut String,
        overall_line_no: u64,
        file_path: P,
    ) -> std::io::Result<()>
    where
        P: Into<PathBuf>,
    {
        let definitions = self.definitions.take().unwrap();
        let mut preprocessor = SleighPreprocessor {
            definitions: Some(definitions),
            compatible: self.compatible,
            file_path: file_path.into(),
            ..Default::default()
        };
        self.definitions = Some(preprocessor.process_internal(writer, overall_line_no)?);
        Ok(())
    }

    pub fn process(&'a mut self, writer: &mut String) -> Definitions {
        // TODO Create error
        self.process_internal(writer, 1).unwrap()
    }

    fn process_internal(
        &'a mut self,
        writer: &mut String,
        overall_line_no: u64,
    ) -> std::io::Result<Definitions> {
        self.line_no = 1;
        self.overall_line_no = overall_line_no;
        self.ifstack
            .push(ConditionalHelper::new(false, false, false, true));

        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        self.output_position(writer);
        trace!("enter SleighPreprocessor");

        for line in reader.lines() {
            let mut line: String = line?;
            trace!("top of while, state: {:?}", self);
            trace!("got line: {}", line);

            let original_line = line.clone();

            // remove confirmed full-line comments
            line = FULL_LINE_COMMENT_RE.replace(&line, "").to_string();

            if !line.is_empty() && line.starts_with('@') {
                // remove any comments in preprocessor
                line = COMMENT_RE.replace(&line, "").to_string();

                if let Some(m) = INCLUDE_RE.captures(&line) {
                    if self.is_copy() {
                        let mut include_file_path =
                            PathBuf::from(self.handle_variables(m.get(1).unwrap().as_str(), true));
                        if include_file_path.is_relative() {
                            include_file_path = PathBuf::from(&self.file_path)
                                .with_file_name(include_file_path.file_name().unwrap());
                        }
                        if !include_file_path.exists() {
                            // TODO Create errors
                            panic!();
                        }
                        self.include_file(writer, self.overall_line_no, include_file_path)?;
                        // increment the position now because we already replaced the include
                        self.line_no += 1;
                        self.overall_line_no += 1;
                        self.output_position(writer);
                        // the one directive we skip printing a blank line
                        continue;
                    }
                } else if let Some(m) = DEFINE1_RE
                    .captures(&line)
                    .or_else(|| DEFINE2_RE.captures(&line))
                {
                    if self.is_copy() {
                        // TODO Create error
                        let key = m.get(1).unwrap().as_str();
                        let value = m.get(2).unwrap().as_str();
                        self.define(key, value);
                    }
                } else if let Some(m) = DEFINE3_RE.captures(&line) {
                    if self.is_copy() {
                        self.define(m.get(1).unwrap().as_str(), "");
                    }
                } else if let Some(m) = UNDEF_RE.captures(&line) {
                    if self.is_copy() {
                        self.undefine(m.get(1).unwrap().as_str());
                    }
                } else if let Some(m) = IFDEF_RE.captures(&line) {
                    self.enter_if();
                    let m = m.get(1).unwrap().as_str();
                    if self.definitions.as_ref().unwrap().contains_key(m) {
                        self.set_handled(true);
                        trace!("@ifdef {}: yes", m);
                    } else {
                        self.set_copy(false);
                        trace!("@ifdef {}: NO", m);
                    }
                } else if let Some(m) = IFNDEF_RE.captures(&line) {
                    self.enter_if();
                    let m = m.get(1).unwrap().as_str();
                    if self.definitions.as_ref().unwrap().contains_key(m) {
                        self.set_copy(false);
                        trace!("@ifndef {}: NO", m);
                    } else {
                        self.set_handled(true);
                        trace!("@ifndef {}: yes", m);
                    }
                } else if let Some(m) = IF_RE.captures(&line) {
                    self.enter_if();
                    trace!("@if... {}", m.get(1).unwrap().as_str());
                // TODO Implement parser of Boolean expressions
                } else if let Some(m) = ELIF_RE.captures(&line) {
                    self.enter_elif();
                    trace!("@elif... {}", m.get(1).unwrap().as_str());
                // TODO Implement parser of Boolean expressions
                } else if ENDIF_RE.is_match(&line) {
                    self.leave_if();
                    trace!("@endif");
                } else if ELSE_RE.is_match(&line) {
                    self.enter_else();
                    self.set_copy(!self.is_handled());
                    trace!("@else");
                } else {
                    // TODO Create error
                    panic!();
                }
                trace!(
                    "PRINT {}: commenting directive out",
                    self.current_position()
                );
                writer.push_str(&format!("# {}\n", original_line));
            } else if self.is_copy() {
                trace!("PRINT {}: printing text", self.current_position());
                writer.push_str(&self.handle_variables(&line, self.compatible));
                writer.push('\n');
            } else {
                trace!(
                    "PRINT {}: replacing text with non-copied blank line",
                    self.current_position()
                );
                writer.push_str(&format!("# {}\n", &line));
            }
            self.line_no += 1;
            self.overall_line_no += 1;
        }
        if self.error_count > 0 {
            // TODO Create errors
            panic!();
        }
        trace!("leave SleighPreprocessor");
        Ok(self.definitions.take().unwrap())
    }

    fn is_copy(&self) -> bool {
        self.ifstack.iter().all(|x| x.copy())
    }

    fn current_position(&self) -> String {
        format!(
            "{}:{}({})",
            self.file_name(),
            self.line_no,
            self.overall_line_no
        )
    }

    fn output_position(&self, writer: &mut String) {
        if !self.compatible {
            let position = format!("\x08{}###{}\x08", self.file_name(), self.line_no);
            writer.push_str(&position);
        }
    }

    fn file_name(&self) -> &str {
        self.file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
    }

    fn handle_variables<S: Into<String>>(&self, input: S, is_compatible: bool) -> String {
        let mut input = input.into();
        let mut output = String::from(&input);
        while let Some(m) = EXPANSION_RE.captures(&input) {
            trace!("current line '{}'", input);
            let expansion = m.get(0).unwrap().as_str();
            trace!("found expansion: {}", expansion);
            let variable = m.get(1).unwrap().as_str();
            let definiton =
                if let Some(definiton) = self.definitions.as_ref().unwrap().get(variable) {
                    definiton
                } else {
                    // TODO Create error
                    panic!();
                };
            if is_compatible {
                output = output.replacen(expansion, definiton, 1);
            } else {
                output =
                    output.replacen(expansion, &format!("\x08{}\x08{}", expansion, definiton), 1);
            }
            input = input.replacen(expansion, "", 1);
        }
        output
    }

    fn define<S>(&mut self, key: S, value: S)
    where
        S: Into<String>,
    {
        let key = key.into();
        let value = value.into();
        trace!("@define {} {}", key, value);
        self.definitions.as_mut().unwrap().insert(key, value);
    }

    fn undefine<S>(&mut self, key: S)
    where
        S: Into<String>,
    {
        let key = key.into();
        trace!("@undef {}", key);
        self.definitions.as_mut().unwrap().remove(&key);
    }

    fn enter_if(&mut self) {
        self.ifstack
            .push(ConditionalHelper::new(true, false, false, self.is_copy()));
    }

    fn leave_if(&mut self) {
        // TODO Create error
        if !self.is_in_if() {
            panic!();
        }
        self.ifstack.pop();
    }

    fn is_in_if(&self) -> bool {
        // TODO Create error
        self.ifstack.last().unwrap().in_if()
    }

    fn enter_elif(&mut self) {
        // TODO Create error
        if !self.is_in_if() {
            panic!();
        }
        if self.is_saw_else() {
            panic!();
        }
    }

    fn enter_else(&mut self) {
        // TODO Create error
        if !self.is_in_if() {
            panic!();
        }
        if self.is_saw_else() {
            panic!();
        }
        self.set_saw_else(true);
    }

    fn set_saw_else(&mut self, is_saw_else: bool) {
        self.ifstack.last_mut().unwrap().set_saw_else(is_saw_else)
    }

    fn is_saw_else(&self) -> bool {
        // TODO Create error
        self.ifstack.last().unwrap().saw_else()
    }

    fn set_copy(&mut self, is_copy: bool) {
        self.ifstack.last_mut().unwrap().set_copy(is_copy);
    }

    fn set_handled(&mut self, is_handled: bool) {
        self.ifstack.last_mut().unwrap().set_handled(is_handled);
    }

    fn is_handled(&self) -> bool {
        self.ifstack.last().unwrap().handled()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
