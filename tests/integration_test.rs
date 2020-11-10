use std::collections::HashMap;
use std::path::PathBuf;

use sleigh_preprocessor::SleighPreprocessor;

fn common(input_name: &str) -> String {
    let mut writer = String::new();
    let mut definitions = HashMap::new();
    definitions.insert("REPLACE".into(), "includes".into());
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(format!("resources/{}.input", input_name));
    let mut sleigh_preprocessor = SleighPreprocessor::new(definitions, path);
    sleigh_preprocessor.process(&mut writer).unwrap();
    writer
}

#[test]
fn a_nestedif2() {
    let writer = common("a_nestedif2");
    let output = include_str!("../resources/a_nestedif2.output");
    assert_eq!(output, writer);
}

#[test]
fn a_simpledefine() {
    let writer = common("a_simpledefine");
    let output = include_str!("../resources/a_simpledefine.output");
    assert_eq!(output, writer);
}

#[test]
fn empty() {
    let writer = common("empty");
    let output = include_str!("../resources/empty.output");
    assert_eq!(output, writer);
}

#[test]
fn expression() {
    let writer = common("expression");
    let output = include_str!("../resources/expression.output");
    assert_eq!(output, writer);
}

#[test]
fn include() {
    let writer = common("include");
    let output = include_str!("../resources/include.output");
    assert_eq!(output, writer);
}

#[test]
fn longertest() {
    let writer = common("longertest");
    let output = include_str!("../resources/longertest.output");
    assert_eq!(output, writer);
}

#[test]
fn nestedif() {
    let writer = common("nestedif");
    let output = include_str!("../resources/nestedif.output");
    assert_eq!(output, writer);
}

#[test]
fn simple() {
    let writer = common("simple");
    let output = include_str!("../resources/simple.output");
    assert_eq!(output, writer);
}

#[test]
fn z_complex() {
    let writer = common("z_complex");
    let output = include_str!("../resources/z_complex.output");
    assert_eq!(output, writer);
}

#[test]
fn regression_oneline_define() {
    let writer = common("oneline_define");
    let output = include_str!("../resources/oneline_define.output");
    assert_eq!(output, writer);
}
