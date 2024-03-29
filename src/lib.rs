use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use log::trace;
use regex::Regex;

pub mod boolean_expression;
mod conditional_helper;
pub mod errors;
pub mod location;

use boolean_expression::parse_boolean_expression;
use conditional_helper::ConditionalHelper;
use errors::{PreprocessorError, Result};
use location::Location;

pub type Definitions = HashMap<String, String>;

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
    locations: Option<Vec<Location>>,
    compatible: bool,

    ifstack: Vec<ConditionalHelper>,
    error_count: u64,

    file_path: PathBuf,
    line: Option<&'a str>,
    line_no: usize,
    overall_line_no: usize,
}

impl<'a> SleighPreprocessor<'a> {
    pub fn new<P>(definitions: Definitions, file_path: P, is_compatible: bool) -> Self
    where
        P: Into<PathBuf>,
    {
        SleighPreprocessor {
            definitions: Some(definitions),
            locations: Some(Vec::new()),
            file_path: file_path.into(),
            compatible: is_compatible,
            ..Default::default()
        }
    }

    pub fn process(&mut self, writer: &mut String) -> Result<()> {
        let (definitions, locations) = self.process_internal(writer, 1)?;
        self.definitions = Some(definitions);
        self.locations = Some(locations);
        Ok(())
    }

    pub fn definitions(&self) -> &Definitions {
        self.definitions.as_ref().unwrap()
    }

    pub fn locations(&self) -> &[Location] {
        self.locations.as_ref().map(|v| v.as_ref()).unwrap()
    }

    pub fn take_definitions(&mut self) -> Definitions {
        self.definitions.take().unwrap()
    }

    pub fn take_locations(&mut self) -> Vec<Location> {
        self.locations.take().unwrap()
    }

    fn include_file(
        &mut self,
        writer: &mut String,
        overall_line_no: usize,
        file_path: impl Into<PathBuf>,
    ) -> Result<()> {
        let definitions = self.definitions.take();
        let locations = self.locations.take();
        let mut preprocessor = SleighPreprocessor {
            compatible: self.compatible,
            file_path: file_path.into(),
            definitions,
            locations,
            ..Default::default()
        };
        let (definitions, locations) = preprocessor.process_internal(writer, overall_line_no)?;
        self.definitions = Some(definitions);
        self.locations = Some(locations);
        Ok(())
    }

    fn process_internal(
        &mut self,
        writer: &mut String,
        overall_line_no: usize,
    ) -> Result<(Definitions, Vec<Location>)> {
        self.line_no = 1;
        self.overall_line_no = overall_line_no;
        self.ifstack
            .push(ConditionalHelper::new(false, false, false, true));

        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        self.output_position(writer, writer.lines().count() + 1);
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
                            PathBuf::from(self.handle_variables(m.get(1).unwrap().as_str(), true)?);
                        if include_file_path.is_relative() {
                            include_file_path = PathBuf::from(&self.file_path)
                                .parent()
                                .unwrap()
                                .join(include_file_path);
                        }
                        if !include_file_path.exists() {
                            return Err(PreprocessorError::new(
                                format!(
                                    "included file \"{}\" does not exist",
                                    include_file_path.display()
                                ),
                                self.file_name(),
                                self.line_no,
                                self.overall_line_no,
                                line,
                            )
                            .into());
                        }
                        self.include_file(writer, self.overall_line_no, include_file_path)?;
                        // increment the position now because we already replaced the include
                        self.line_no += 1;
                        self.overall_line_no += 1;
                        self.output_position(writer, writer.lines().count() + 1);
                        // the one directive we skip printing a blank line
                        continue;
                    }
                } else if let Some(m) = DEFINE1_RE
                    .captures(&line)
                    .or_else(|| DEFINE2_RE.captures(&line))
                {
                    if self.is_copy() {
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
                    let m = m.get(1).unwrap().as_str();
                    trace!("@if... {}", m);
                    self.handle_expression(m)?;
                } else if let Some(m) = ELIF_RE.captures(&line) {
                    self.enter_elif(&line)?;
                    let m = m.get(1).unwrap().as_str();
                    trace!("@elif... {}", m);
                    self.handle_expression(m)?;
                } else if ENDIF_RE.is_match(&line) {
                    self.leave_if(&line)?;
                    trace!("@endif");
                } else if ELSE_RE.is_match(&line) {
                    self.enter_else(line)?;
                    self.set_copy(!self.is_handled());
                    trace!("@else");
                } else {
                    return Err(PreprocessorError::new(
                        "unrecognized preprocessor directive",
                        self.file_name(),
                        self.line_no,
                        self.overall_line_no,
                        &line,
                    )
                    .into());
                }
                trace!(
                    "PRINT {}: commenting directive out",
                    self.current_position()
                );
                writer.push_str(&format!("#{}\n", original_line));
            } else if self.is_copy() {
                trace!("PRINT {}: printing text", self.current_position());
                writer.push_str(&self.handle_variables(&line, self.compatible)?);
                writer.push('\n');
            } else {
                trace!(
                    "PRINT {}: replacing text with non-copied blank line",
                    self.current_position()
                );
                writer.push_str(&format!("#{}\n", &line));
            }
            self.line_no += 1;
            self.overall_line_no += 1;
        }
        if self.error_count > 0 {
            return Err(PreprocessorError::new(
                "Error during preprocessing",
                self.file_name(),
                self.overall_line_no,
                0,
                "",
            )
            .into());
        }
        trace!("leave SleighPreprocessor");
        Ok((
            self.definitions.take().unwrap(),
            self.locations.take().unwrap(),
        ))
    }

    fn current_position(&self) -> String {
        format!(
            "{}:{}({})",
            self.file_name(),
            self.line_no,
            self.overall_line_no
        )
    }

    fn output_position(&mut self, writer: &mut String, all_line_count: usize) {
        let file_name = self.file_name().to_string();
        let line_no = self.line_no;
        if !self.compatible {
            let position = format!("\x08{}###{}\x08", file_name, line_no);
            writer.push_str(&position);
        }
        self.locations
            .as_mut()
            .unwrap()
            .push(Location::new(&self.file_path, line_no, all_line_count));
    }

    fn file_name(&self) -> &str {
        self.file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
    }

    fn handle_expression<S: AsRef<str>>(&mut self, expression: S) -> Result<()> {
        let expression = expression.as_ref();
        if self.is_handled() {
            self.set_copy(false);
            trace!("already handled");
        } else if !self.parse_expression(expression)? {
            self.set_copy(false);
            trace!("expression \"{}\" is FALSE", expression);
        } else {
            self.set_copy(true);
            self.set_handled(true);
            trace!("expression \"{}\" is true", expression);
        }
        Ok(())
    }

    fn parse_expression<S: AsRef<str>>(&self, expression: S) -> Result<bool> {
        let expression = expression.as_ref();
        parse_boolean_expression(expression, &self.definitions.as_ref().unwrap()).map_err(|e| {
            PreprocessorError::new(
                format!("parser error: {}", e),
                self.file_name(),
                self.line_no,
                self.overall_line_no,
                expression.to_string(),
            )
            .into()
        })
    }

    fn handle_variables<S: Into<String>>(&self, input: S, is_compatible: bool) -> Result<String> {
        let mut input = input.into();
        let mut output = String::new();
        while let Some(m) = EXPANSION_RE.captures(&input) {
            trace!("current line '{}'", input);
            let expansion_match = m.get(0).unwrap();
            let expansion = expansion_match.as_str();
            trace!("found expansion: {}", expansion);
            let variable = m.get(1).unwrap().as_str();
            let definiton = self
                .definitions
                .as_ref()
                .unwrap()
                .get(variable)
                .ok_or_else(|| {
                    errors::Error::Preprocessor(PreprocessorError::new(
                        format!("unknown variable: {}", variable),
                        self.file_name(),
                        self.line_no,
                        self.overall_line_no,
                        input.to_string(),
                    ))
                })?;
            output.push_str(input.get(0..expansion_match.start()).unwrap());
            if !is_compatible {
                output.push('\x08');
                output.push_str(expansion);
                output.push('\x08');
            }
            output.push_str(definiton);
            input = input.get(expansion_match.end()..).unwrap().to_string();
        }
        output.push_str(&input);
        Ok(output)
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

    fn enter_elif<S: AsRef<str>>(&mut self, line: S) -> Result<()> {
        if !self.is_in_if() {
            return Err(PreprocessorError::new(
                "elif outside of IF* directive",
                self.file_name(),
                self.line_no,
                self.overall_line_no,
                line.as_ref(),
            )
            .into());
        }
        if self.is_saw_else() {
            return Err(PreprocessorError::new(
                "already saw else directive",
                self.file_name(),
                self.line_no,
                self.overall_line_no,
                line.as_ref(),
            )
            .into());
        }
        Ok(())
    }

    fn leave_if<S: AsRef<str>>(&mut self, line: S) -> Result<()> {
        if !self.is_in_if() {
            return Err(PreprocessorError::new(
                "not in IF* directive",
                self.file_name(),
                self.line_no,
                self.overall_line_no,
                line.as_ref(),
            )
            .into());
        }
        self.ifstack.pop();
        Ok(())
    }

    fn enter_else<S: AsRef<str>>(&mut self, line: S) -> Result<()> {
        if !self.is_in_if() {
            return Err(PreprocessorError::new(
                "else outside of IF* directive",
                self.file_name(),
                self.line_no,
                self.overall_line_no,
                line.as_ref(),
            )
            .into());
        }
        if self.is_saw_else() {
            return Err(PreprocessorError::new(
                "duplicate else directive",
                self.file_name(),
                self.line_no,
                self.overall_line_no,
                line.as_ref(),
            )
            .into());
        }
        self.set_saw_else(true);
        Ok(())
    }

    // Functions for checking/setting the ifstack. The ifstack always must be not empty.

    fn is_in_if(&self) -> bool {
        self.ifstack.last().unwrap().in_if()
    }

    fn set_saw_else(&mut self, is_saw_else: bool) {
        self.ifstack.last_mut().unwrap().set_saw_else(is_saw_else)
    }

    fn is_saw_else(&self) -> bool {
        self.ifstack.last().unwrap().saw_else()
    }

    fn set_copy(&mut self, is_copy: bool) {
        self.ifstack.last_mut().unwrap().set_copy(is_copy);
    }

    fn is_copy(&self) -> bool {
        self.ifstack.iter().all(|x| x.copy())
    }

    fn set_handled(&mut self, is_handled: bool) {
        self.ifstack.last_mut().unwrap().set_handled(is_handled);
    }

    fn is_handled(&self) -> bool {
        self.ifstack.last().unwrap().handled()
    }
}
