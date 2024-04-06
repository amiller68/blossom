use quick_xml::de::from_str;
use serde::Deserialize;

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Arg {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@type")]
    r#type: String,
    #[serde(rename = "@value")]
    value: String,
}

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AgentCall {
    #[serde(rename = "@name")]
    name: String,
    arg: Arg,
}

#[cfg(test)]
mod test {
    use super::*;
    const XML: &str = r#"
<?xml version="1.0" encoding="utf-8"?>
<agent-call name="image">
    <arg name="path" type="PathBuf" value="images/1.jpg"/>
    <arg name="path" value="images/1.jpg"/>
</agent-call>
"#;
    #[test]
    fn test_deserialize() {
        let agent_call: AgentCall = from_str(XML).unwrap();
        assert_eq!(
            agent_call,
            AgentCall {
                name: "image".to_string(),
                arg: Arg {
                    name: "path".to_string(),
                    r#type: "PathBuf".to_string(),
                    value: "images/1.jpg".to_string(),
                },
            }
        );
    }
}
