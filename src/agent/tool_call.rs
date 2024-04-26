#[allow(unused_imports)]
pub use quick_xml::de::from_str;
use serde::Deserialize;

#[derive(Debug, PartialEq, Default, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Argument {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@type")]
    r#type: String,
    #[serde(rename = "@value")]
    value: String,
}

impl Argument {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn r#type(&self) -> &str {
        &self.r#type
    }
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ToolCall {
    #[serde(rename = "@name")]
    name: String,
    argument: Vec<Argument>,
}

impl ToolCall {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn args(&self) -> &[Argument] {
        &self.argument
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ToolCallError {
    #[error("Parse error: {0}")]
    ParseError(#[from] quick_xml::de::DeError),
}

impl TryFrom<&str> for ToolCall {
    type Error = ToolCallError;
    fn try_from(xml: &str) -> Result<Self, Self::Error> {
        from_str(xml).map_err(ToolCallError::ParseError)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const XML: &str = r#"
<?xml version="1.0" encoding="utf-8"?>
<tool-call name="image">
    <argument name="path" type="PathBuf" value="images/1.jpg"/>
</tool-call>
"#;
    #[test]
    fn test_deserialize() {
        let tool_call = ToolCall::try_from(XML).unwrap();
        assert_eq!(
            tool_call,
            ToolCall {
                name: "image".to_string(),
                argument: [Argument {
                    name: "path".to_string(),
                    r#type: "PathBuf".to_string(),
                    value: "images/1.jpg".to_string(),
                }]
                .to_vec()
            }
        );
    }
}
