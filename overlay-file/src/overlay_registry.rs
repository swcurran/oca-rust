use crate::OverlayFile;
use crate::{OverlayDef, parse_from_string};
use log::debug;
use std::{collections::HashMap, fs, path::Path};

pub trait OverlayRegistry {
    fn get_by_filename(&self, name: &str) -> Option<&OverlayFile>;
    /// Get overlay by name or fully qualified name
    /// Supports: name, namespace:name, name/version, namespace:name/version
    fn get_overlay(&self, name: &str) -> Result<&OverlayDef, &'static str>;
    fn list_by_namespace(&self, namespace: &str) -> Vec<&OverlayFile>;

    fn list_all(&self) -> Vec<String>;
}

#[derive(Debug, Clone)]
/// Simple registry for overlays providing read from file, dir and string
pub struct OverlayLocalRegistry {
    overlays: HashMap<String, OverlayFile>,
}

impl OverlayLocalRegistry {
    pub fn from_string(content: String) -> Result<Self, String> {
        let mut overlays = HashMap::new();

        let overlay_file = parse_from_string(content.clone()).map_err(|e| {
            format!(
                "Failed to parse overlay definition, error '{}' from string '{}'",
                content, e
            )
        })?;

        debug!(
            "Loaded overlay definition '{}' with {} overlay(s)",
            content,
            overlay_file.overlays_def.len()
        );
        // TODO convert overlays to vec not need for hash
        overlays.insert("string".to_string(), overlay_file);

        Ok(OverlayLocalRegistry { overlays })
    }

    pub fn from_dir<P: AsRef<Path>>(dir: P) -> Result<Self, std::io::Error> {
        let mut overlays = HashMap::new();

        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("overlayfile")
                && let Some(name) = Self::overlay_name_from_path(&path)
            {
                let content = fs::read_to_string(&path)?;
                debug!("Parsing overlay file: {}", path.display());
                let schema = parse_from_string(content);
                overlays.insert(name, schema.unwrap());
            }
        }

        Ok(OverlayLocalRegistry { overlays })
    }

    pub fn from_file<P: AsRef<Path>>(file: P) -> Result<Self, std::io::Error> {
        let path = file.as_ref();

        // Ensure it’s an overlay file
        if path.extension().and_then(|s| s.to_str()) != Some("overlayfile") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "File does not have .overlayfile extension",
            ));
        }

        let mut overlays = HashMap::new();

        if let Some(name) = Self::overlay_name_from_path(path) {
            debug!("Parsing overlay file: {}", path.display());
            let content = fs::read_to_string(path)?;
            let schema = parse_from_string(content);
            overlays.insert(name, schema.unwrap());
        }

        Ok(OverlayLocalRegistry { overlays })
    }

    fn overlay_name_from_path(path: &Path) -> Option<String> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }

    pub fn new() -> Self {
        OverlayLocalRegistry {
            overlays: HashMap::new(),
        }
    }
}

impl Default for OverlayLocalRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayRegistry for OverlayLocalRegistry {
    // TODO remove it shouldn't be needed
    fn get_by_filename(&self, name: &str) -> Option<&OverlayFile> {
        self.overlays.get(name)
    }

    fn get_overlay(&self, overlay_name: &str) -> Result<&OverlayDef, &'static str> {
        // Remove "overlay/" prefix if present, as it should not be used to search for in registry
        let overlay_name = overlay_name
            .strip_prefix("overlay/")
            .unwrap_or(overlay_name);
        debug!("Getting overlay by fq name: {}", overlay_name);

        // Parse namespace if present (format: namespace:name or name)
        let (namespace, remaining) = overlay_name
            .split_once(':')
            .map(|(ns, n)| (Some(ns.to_ascii_lowercase()), n))
            .unwrap_or((None, overlay_name));

        // Parse version if present (format: name/version or name)
        let (name, version) = remaining
            .split_once('/')
            .map(|(n, v)| (n, Some(v)))
            .unwrap_or((remaining, None));

        let name = name.to_ascii_lowercase();

        // Find matching overlay
        let candidates: Vec<&OverlayDef> = self
            .overlays
            .values()
            .flat_map(|overlay_file| &overlay_file.overlays_def)
            .filter(|o| {
                let o_ns = o.namespace.as_ref().map(|s| s.to_ascii_lowercase());
                let name_matches = o.name.eq_ignore_ascii_case(&name);

                // Check namespace match
                let namespace_matches = match (&namespace, &o_ns) {
                    (Some(ns), Some(o_ns)) => ns == o_ns,
                    (None, _) => true,        // No namespace specified, match any
                    (Some(_), None) => false, // Namespace specified but overlay has none
                };

                // Check version match
                let version_matches = match &version {
                    Some(v) => o.version.eq_ignore_ascii_case(v),
                    None => true, // No version specified, match any
                };

                name_matches && namespace_matches && version_matches
            })
            .collect();

        match candidates.len() {
            0 => Err("Overlay definition not found in registry"),
            1 => Ok(candidates[0]),
            _ => {
                // Multiple matches found - this happens when namespace or version is not specified
                // Return the first match, but log a warning
                debug!(
                    "Multiple overlays found for '{}'. Returning first match. Consider specifying namespace and/or version.",
                    overlay_name
                );
                Ok(candidates[0])
            }
        }
    }

    fn list_all(&self) -> Vec<String> {
        // Extract all overlay namespace
        self.overlays
            .iter()
            .flat_map(|(_, overlay_file)| {
                overlay_file.overlays_def.iter().map(|o| {
                    let namespace = o
                        .namespace
                        .as_ref()
                        .map_or(String::new(), |ns| format!("{:?}:", ns));
                    format!("{}{}/{}", namespace, o.name, o.version)
                })
            })
            .collect::<Vec<String>>()
    }

    fn list_by_namespace(&self, _namespace: &str) -> Vec<&OverlayFile> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_registry() {
        let _ = env_logger::builder().is_test(true).try_init();
        let registry = OverlayLocalRegistry::from_dir("core_overlays").unwrap();
        assert_eq!(registry.list_all().len(), 13);
        assert!(registry.get_by_filename("semantic").is_some());
        assert_eq!(registry.get_overlay("label/2.0.0").unwrap().name, "label");

        // File can include more then one overlay
        let semantic_overlay_file = registry.get_by_filename("semantic").unwrap();
        assert_eq!(semantic_overlay_file.overlays_def.len(), 13);
        let label_overlay = semantic_overlay_file.overlays_def.first().unwrap();
        assert_eq!(label_overlay.name, "label");
    }
    #[test]
    fn test_overlay_not_found_by_name() {
        let _ = env_logger::builder().is_test(true).try_init();
        let registry = OverlayLocalRegistry::from_dir("core_overlays").unwrap();

        let result = registry.get_overlay("nonexistent_overlay");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Overlay definition not found in registry"
        );
    }

    #[test]
    fn test_overlay_with_namespace_by_name() {
        let _ = env_logger::builder().is_test(true).try_init();

        let registry = OverlayLocalRegistry::from_dir("test_overlays").unwrap();

        let result = registry.get_overlay("hcf:information");
        assert!(result.is_ok());
        let overlay = result.unwrap();
        assert_eq!(overlay.name, "information");
        assert_eq!(overlay.namespace, Some("hcf".to_string()));

        // Test retrieving by name without namespace (should still work if unique)
        let result = registry.get_overlay("information");
        assert!(result.is_ok());

        // Test non-existent namespaced overlay
        let result = registry.get_overlay("nonexistent:overlay");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Overlay definition not found in registry"
        );
    }
}
