use glob::glob;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::{self, create_dir_all};
use std::path::{Path, PathBuf};

use crate::error::{CodegenError, Result};

use crate::fragment::Fragment;

use crate::generator::TypeScriptGenerator;

pub struct AbiProcessor {
    out_dir: PathBuf,
    abis_by_file: HashMap<PathBuf, Vec<Fragment>>,
}

impl AbiProcessor {
    pub fn new(out_dir: &str) -> Self {
        Self {
            out_dir: PathBuf::from(out_dir),
            abis_by_file: HashMap::new(),
        }
    }

    pub fn collect_abi_files(&mut self, source_pattern: &str) -> Result<()> {
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

    pub fn extract_abis_from_file(&self, path: &Path) -> Result<(PathBuf, Vec<Fragment>)> {
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

    pub fn deduplicate_abis(&self, abis: Vec<Fragment>) -> Vec<Fragment> {
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

    pub fn generate_typescript_files(&self) -> Result<()> {
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
