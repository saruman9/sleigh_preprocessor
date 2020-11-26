use std::collections::HashMap;

use sleigh_preprocessor::boolean_expression::parse_boolean_expression;
use sleigh_preprocessor::errors::Error;

#[test]
fn eq_or() {
    let input = r#"SH_VERSION == "2" || SH_VERSION == "2A""#;
    let mut definitions = HashMap::new();
    definitions.insert("SH_VERSION".to_string(), "2".to_string());
    assert!(parse_boolean_expression(input, &definitions).unwrap());
}

#[test]
fn paren() {
    let input = "(defined(dsPIC24F) || defined(dsPIC33E)) && defined(dsPIC33C)";
    let mut definitions = HashMap::new();
    definitions.insert("dsPIC24F".to_string(), Default::default());
    assert!(!parse_boolean_expression(input, &definitions).unwrap());
}

#[test]
fn not_defined() {
    let input = r#"(defined(test) || defined(ti)) && PROCESSOR == "PIC_16""#;
    assert_eq!(
        parse_boolean_expression(input, &Default::default())
            .err()
            .unwrap(),
        Error::NotDefined("PROCESSOR".to_string())
    );
}

#[test]
fn definitions_eq() {
    let input = r#"DATA_ENDIAN == "little""#;
    let mut definitions = HashMap::new();
    definitions.insert("DATA_ENDIAN".to_string(), "little".to_string());
    assert!(parse_boolean_expression(input, &definitions).unwrap());
}

#[test]
fn equals() {
    let input = r#""FOO" == "FOO""#;
    assert!(parse_boolean_expression(input, &Default::default()).unwrap());
}

#[test]
fn no_equals() {
    let input = r#""BOO" == "FOO""#;
    assert!(!parse_boolean_expression(input, &Default::default()).unwrap());
}

#[test]
fn definitions_no_eq() {
    let input = r#""BOO" == "FOO""#;
    let mut definitions = HashMap::new();
    definitions.insert("BOO".to_string(), "TEST".to_string());
    assert!(!parse_boolean_expression(input, &definitions).unwrap());
}

#[test]
fn more_parens() {
    let input = r#"((((FOO == FOO))))"#;
    let mut definitions = HashMap::new();
    definitions.insert("FOO".to_string(), "FOO".to_string());
    assert!(parse_boolean_expression(input, &definitions).unwrap());
}

#[test]
fn not_inv() {
    let input = r#"!(FOO != FOO)"#;
    let mut definitions = HashMap::new();
    definitions.insert("FOO".to_string(), "FOO".to_string());
    assert!(parse_boolean_expression(input, &definitions).unwrap());
}

#[test]
fn more_nots() {
    let input = r#"!(!(!(FOO == FOO)))"#;
    let mut definitions = HashMap::new();
    definitions.insert("FOO".to_string(), "FOO".to_string());
    assert!(!parse_boolean_expression(input, &definitions).unwrap());
}

#[test]
fn even_more_nots() {
    let input = r#"!(!(!(!(FOO == FOO))))"#;
    let mut definitions = HashMap::new();
    definitions.insert("FOO".to_string(), "FOO".to_string());
    assert!(parse_boolean_expression(input, &definitions).unwrap());
}

#[test]
fn complicated() {
    let input = r#"!(A!="A" || B=="B" ^^ (C!="C" || D=="D") && E!="E")"#;
    let mut definitions = HashMap::new();
    definitions.insert("A".to_string(), "A".to_string());
    definitions.insert("B".to_string(), "B".to_string());
    definitions.insert("C".to_string(), "C".to_string());
    definitions.insert("D".to_string(), "D".to_string());
    definitions.insert("E".to_string(), "E".to_string());
    assert!(!parse_boolean_expression(input, &definitions).unwrap());
}
