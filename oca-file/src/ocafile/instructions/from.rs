use crate::ocafile::{Pair, Rule, error::InstructionError};
use log::debug;
use oca_ast::ast::{BundleContent, Command, CommandType, ObjectKind, RefValue, ReferenceAttrType};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FromInstruction {}

impl FromInstruction {
    pub(crate) fn from_record(record: Pair, _index: usize) -> Result<Command, InstructionError> {
        let mut reference: Option<ReferenceAttrType> = None;

        for field in record.into_inner() {
            let said = match field.as_rule() {
                Rule::said => {
                    let said_str = field.as_str();

                    let said = said_str
                        .to_string()
                        .parse()
                        .map_err(|e| InstructionError::Parser(format!("Invalid SAID: {}", e)))?;
                    debug!("FROM SAID: {:?}", said);
                    ReferenceAttrType::Reference(RefValue::Said(said))
                }
                Rule::alias => {
                    debug!("Using oca bundle by name: {:?}", field);
                    ReferenceAttrType::Reference(RefValue::Name(field.to_string()))
                }
                _ => {
                    return Err(InstructionError::Parser(format!(
                        "Invalid reference: {:?}",
                        field.as_rule()
                    )));
                }
            };

            reference = Some(said);
        }

        let reference = reference.ok_or_else(|| {
            InstructionError::Parser("Missing reference (expected said or alias)".to_string())
        })?;

        Ok(Command {
            kind: CommandType::From,
            object_kind: ObjectKind::OCABundle(BundleContent { said: reference }),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ocafile::{self, OCAfileParser, Pair, Rule, error::InstructionError};
    use oca_ast::ast::RefValue;
    use pest::Parser;

    pub fn parse_direct<T, F>(input: &str, rule: Rule, func: F) -> Result<T, InstructionError>
    where
        F: Fn(Pair) -> Result<T, InstructionError>,
    {
        let mut pairs = OCAfileParser::parse(rule, input)
            .map_err(|e| InstructionError::Parser(format!("Parse error: {}", e)))?;
        let pair = pairs.next().ok_or(InstructionError::UnexpectedToken(
            "Unknown parser error".to_string(),
        ))?;

        func(pair)
    }

    use super::*;

    #[test]
    fn test_from_instruction() -> Result<(), InstructionError> {
        let _ = env_logger::builder().is_test(true).try_init();
        // test vector with example instruction and boolean if they should be valid or not
        let instructions = vec![
            (
                "FROM refs:ENmwqnqVxonf_bNZ0hMipOJJY25dxlC8eSY5BbyMCfLJ",
                true,
            ),
            (
                "from refs:ENmwqnqVxonf_bNZ0hMipOJJY25dxlC8eSY5BbyMCfLJ",
                true,
            ),
            ("from error", false),
            ("from refn:local-oca-name", true),
            ("from local-oca-name", false),
            (
                "from https://humancolossus.org/ENmwqnqVxonf_bNZ0hMipOJJY25dxlC8eSY5BbyMCfLJ",
                false,
            ),
        ];

        for (instruction, is_valid) in instructions {
            let result = parse_direct(instruction, Rule::from, |p| {
                FromInstruction::from_record(p, 0)
            });

            debug!("Processing instruction: {:?}", result);
            match result {
                Ok(command) => {
                    let content = command.object_kind.oca_bundle_content().unwrap();

                    match content.clone().said {
                        ocafile::ast::ReferenceAttrType::Reference(refs) => match refs {
                            RefValue::Said(_said) => {
                                assert!(is_valid, "Instruction should be valid");
                            }
                            RefValue::Name(_) => {
                                assert!(is_valid, "Instruction should be valid");
                            }
                        },
                    }
                }
                Err(_e) => {
                    assert!(!is_valid, "Instruction should be invalid")
                }
            }
        }
        Ok(())
    }
}
