use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::error::Result;
use crate::fragment::Fragment;

pub struct TypeScriptGenerator;

impl TypeScriptGenerator {
    pub fn create_literal_for(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => format!("\"{}\"", s.replace('\"', "\\\"")),
            Value::Array(arr) => {
                let elements: Vec<String> =
                    arr.iter().map(|e| Self::create_literal_for(e)).collect();
                format!("[{}]", elements.join(", "))
            }
            Value::Object(obj) => {
                let properties: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, Self::create_literal_for(v)))
                    .collect();
                format!("{{{}}}", properties.join(", "))
            }
        }
    }

    pub fn generate_fragment_declaration(
        fragment: &Fragment,
        use_explicit_identifier: bool,
    ) -> Result<(String, String)> {
        let identifier = fragment.identifier(use_explicit_identifier)?;

        // Convert fragment to JSON Value for serialization
        let fragment_value = serde_json::to_value(fragment)?;
        let object_literal = Self::create_literal_for(&fragment_value);

        let declaration = format!("export const {} = {} as const;", identifier, object_literal);

        Ok((identifier, declaration))
    }

    pub fn generate_file_content(fragments: Vec<Fragment>) -> Result<Option<String>> {
        let filtered_fragments: Vec<Fragment> = fragments
            .into_iter()
            .filter(|fragment| {
                fragment.name.is_some() && !fragment.name.as_ref().unwrap().is_empty()
            })
            .collect();

        if filtered_fragments.is_empty() {
            return Ok(None);
        }

        // Count fragment names for disambiguation
        let name_counts: HashMap<String, usize> = filtered_fragments
            .iter()
            .filter_map(|f| f.name.clone())
            .fold(HashMap::new(), |mut counts, name| {
                *counts.entry(name).or_insert(0) += 1;
                counts
            });

        let mut identifiers = Vec::new();
        let mut declarations = Vec::new();
        let mut processed_fragments = HashSet::new();

        for fragment in filtered_fragments {
            let fragment_key = fragment.get_unique_key();

            if processed_fragments.contains(&fragment_key) {
                continue;
            }
            processed_fragments.insert(fragment_key);

            let name = fragment.name.clone().unwrap();
            let use_explicit_identifier = name_counts.get(&name).unwrap_or(&0) > &1;

            let (identifier, declaration) =
                Self::generate_fragment_declaration(&fragment, use_explicit_identifier)?;

            identifiers.push(identifier);
            declarations.push(declaration);
        }

        if identifiers.is_empty() {
            return Ok(None);
        }

        let export_default = format!("export default [{}] as const;", identifiers.join(", "));

        let file_content = format!("{}\n\n{}", declarations.join("\n\n"), export_default);

        Ok(Some(file_content))
    }
}
