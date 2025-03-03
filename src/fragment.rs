use crate::error::{CodegenError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct FragmentInput {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(rename = "type")]
    pub type_name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indexed: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct FragmentOutput {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(rename = "type")]
    pub type_name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Fragment {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(rename = "type")]
    pub type_name: String,

    pub inputs: Vec<FragmentInput>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<FragmentOutput>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_mutability: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anonymous: Option<bool>,
}

impl Fragment {
    pub fn get_unique_key(&self) -> String {
        let name = self.name.clone().unwrap_or_default();

        let mut input_types: Vec<String> = self
            .inputs
            .iter()
            .map(|input| input.type_name.clone())
            .collect();
        input_types.sort();

        let mut output_types: Vec<String> = if let Some(outputs) = &self.outputs {
            outputs
                .iter()
                .map(|output| output.type_name.clone())
                .collect()
        } else {
            Vec::new()
        };
        output_types.sort();

        format!(
            "{}:{}:{}:{}",
            name,
            self.type_name,
            input_types.join(","),
            output_types.join(",")
        )
    }

    pub fn identifier(&self, use_explicit_identifier: bool) -> Result<String> {
        let name = self
            .name
            .clone()
            .filter(|n| !n.is_empty())
            .ok_or(CodegenError::MissingName)?;

        if !use_explicit_identifier {
            return Ok(name);
        }

        let input_types = self
            .inputs
            .iter()
            .map(|input| input.type_name.replace("[]", "Array"))
            .collect::<Vec<String>>()
            .join("_");

        Ok(format!("{}_{}", name, input_types))
    }
}
