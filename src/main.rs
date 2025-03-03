use clap::Parser;
use glob::glob;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::{self, create_dir_all};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CodegenError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Glob pattern error: {0}")]
    Glob(#[from] glob::PatternError),

    #[error("Missing fragment name")]
    MissingName,

    #[error("Processing error: {0}")]
    Processing(String),
}

type Result<T> = std::result::Result<T, CodegenError>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
struct FragmentInput {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "type")]
    type_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    indexed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    internal_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
struct FragmentOutput {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "type")]
    type_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    internal_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
struct Fragment {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "type")]
    type_name: String,
    inputs: Vec<FragmentInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    outputs: Option<Vec<FragmentOutput>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    state_mutability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    anonymous: Option<bool>,
}

#[derive(Parser, Debug)]
#[command(version, about = "ABI to TypeScript code generator")]
struct Args {
    /// Source directory containing JSON ABI files
    #[arg(short, long, required = true)]
    source: String,

    /// Output directory for TypeScript files
    #[arg(short, long, default_value = "./abis")]
    out_dir: String,
}

impl Fragment {
    fn get_unique_key(&self) -> String {
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

    fn identifier(&self, use_explicit_identifier: bool) -> Result<String> {
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

struct TypeScriptGenerator;

impl TypeScriptGenerator {
    fn create_literal_for(value: &Value) -> String {
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

    fn generate_fragment_declaration(
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

    fn generate_file_content(fragments: Vec<Fragment>) -> Result<Option<String>> {
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

struct AbiProcessor {
    out_dir: PathBuf,
    abis_by_file: HashMap<PathBuf, Vec<Fragment>>,
}

impl AbiProcessor {
    fn new(out_dir: &str) -> Self {
        Self {
            out_dir: PathBuf::from(out_dir),
            abis_by_file: HashMap::new(),
        }
    }

    fn collect_abi_files(&mut self, source_pattern: &str) -> Result<()> {
        let entries: Vec<PathBuf> = glob(source_pattern)?
            .filter_map(|result| result.ok())
            .filter(|path| !path.to_string_lossy().ends_with(".dbg.json"))
            .collect();

        // Process files in parallel
        let results: Vec<Result<(PathBuf, Vec<Fragment>)>> = entries
            .par_iter()
            .map(|entry| self.extract_abis_from_file(entry))
            .collect();

        // Combine results
        for result in results {
            match result {
                Ok((output_path, abis)) => {
                    if !abis.is_empty() {
                        self.abis_by_file
                            .entry(output_path)
                            .or_insert_with(Vec::new)
                            .extend(abis);
                    }
                }
                Err(e) => eprintln!("Error processing file: {}", e),
            }
        }

        Ok(())
    }

    fn extract_abis_from_file(&self, path: &Path) -> Result<(PathBuf, Vec<Fragment>)> {
        let file_content = fs::read_to_string(path)?;
        let data: Value = serde_json::from_str(&file_content)?;

        let abis = if let Some(Value::Array(abi_values)) = data.get("abi") {
            abi_values
                .iter()
                .filter_map(|v| serde_json::from_value::<Fragment>(v.clone()).ok())
                .collect()
        } else {
            Vec::new()
        };

        let file_name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let output_path = self.out_dir.join(format!("{}.ts", file_name));

        Ok((output_path, abis))
    }

    fn deduplicate_abis(&self, abis: Vec<Fragment>) -> Vec<Fragment> {
        let mut unique_abis = Vec::new();
        let mut seen = HashSet::new();

        for abi in abis {
            let key = abi.get_unique_key();
            if !seen.contains(&key) {
                seen.insert(key);
                unique_abis.push(abi);
            }
        }

        unique_abis
    }

    fn generate_typescript_files(&self) -> Result<()> {
        // Create output directory if it doesn't exist
        create_dir_all(&self.out_dir)?;

        // Generate TypeScript files in parallel
        self.abis_by_file
            .par_iter()
            .try_for_each(|(output_path, abis)| {
                let unique_abis = self.deduplicate_abis(abis.clone());

                match TypeScriptGenerator::generate_file_content(unique_abis)? {
                    Some(content) => fs::write(output_path, content)?,
                    None => (), // Skip empty files
                }

                Ok::<(), CodegenError>(())
            })?;

        Ok(())
    }
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Setup processor
    let mut processor = AbiProcessor::new(&args.out_dir);

    // Process source files
    let source_pattern = format!("{}/**/*.json", args.source);
    processor.collect_abi_files(&source_pattern)?;

    // Generate TypeScript files
    processor.generate_typescript_files()?;

    Ok(())
}
