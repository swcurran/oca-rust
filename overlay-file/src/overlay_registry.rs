use crate::{parse_from_string, OverlayDef};
use std::{collections::HashMap, fs, path::{Path, PathBuf}};
use crate::OverlayFile;
use log::debug;

pub trait OverlayRegistry {
    fn get_by_filename(&self, name: &str) -> Option<&OverlayFile>;
    /// Get overlay by name (namespace + name)
    fn get_by_name(&self, name: &str) -> Result<Option<&OverlayDef>, &'static str>;
    /// Get overlay by fully qualified name (namespace + name + version)
    fn get_by_fqn(&self, name: &str) -> Result<Option<&OverlayDef>, &'static str>;
    fn list_by_namespace(&self, namespace: &str) -> Vec<&OverlayFile>;

    fn list_all(&self) -> Vec<String>;
}

#[derive(Debug, Clone)]
/// File based registry for overlays
pub struct OverlayLocalRegistry {
    overlays: HashMap<String, OverlayFile>,
}

impl OverlayLocalRegistry {
    pub fn from_dir<P: AsRef<Path>>(dir: P) -> Result<Self, std::io::Error> {
        let mut overlays = HashMap::new();

        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("overlayfile") {
                if let Some(name) = Self::overlay_name_from_path(&path) {
                    let content = fs::read_to_string(&path)?;
                    let schema = parse_from_string(content);
                    overlays.insert(name, schema.unwrap());
                }
            }
        }

        Ok(OverlayLocalRegistry { overlays })
    }

    fn overlay_name_from_path(path: &PathBuf) -> Option<String> {
        path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())
    }

    pub fn new() -> Self {
        OverlayLocalRegistry {
            overlays: HashMap::new(),
        }
    }
}

impl OverlayRegistry for OverlayLocalRegistry {
    // TODO remove it shouldn't be needed
    fn get_by_filename(&self, name: &str) -> Option<&OverlayFile> {
        self.overlays.get(name)
    }

    fn get_by_fqn(&self, overlay_name: &str) -> Result<Option<&OverlayDef>, &'static str> {
        debug!("Getting overlay by fq name: {}", overlay_name);
        let (namespace, name) = overlay_name.split_once(':').map(|(ns, n)| (Some(ns), n)).unwrap_or((None, overlay_name));
        let (name, version) = name.split_once("/").ok_or_else(|| "Invalid overlay name format: version not found or in wrong format")?;
        let name = name.to_ascii_lowercase();
        let namespace = namespace.map(|ns| ns.to_ascii_lowercase());

        let overlay_def = self.overlays.values().find_map(|overlay_file| {
            overlay_file.overlays_def.iter().find(|o| {
                let o_ns = o.namespace.as_ref().map(|s| s.to_ascii_lowercase());
                o_ns == namespace && o.name.eq_ignore_ascii_case(&name)
                && o.version.eq_ignore_ascii_case(version)
            })
        });
        Ok(overlay_def)
    }

    fn get_by_name(&self, name: &str) -> Result<Option<&OverlayDef>, &'static str> {
        debug!("Getting overlay by name: {}", name);
        let overlay_def = self.overlays.values().find_map(|overlay_file| {
            overlay_file.overlays_def.iter().find(|o| {
                o.name.eq_ignore_ascii_case(name)
            })
        });
        Ok(overlay_def)
    }


    fn list_all(&self) -> Vec<String> {
        // Extract all overlay namespace
        self.overlays
            .iter()
            .flat_map(|(_, overlay_file)| {
                overlay_file.overlays_def.iter().map(|o| {
                    let namespace = o.namespace
                        .as_ref()
                        .map_or(String::new(), |ns| format!("{:?}:", ns));
                    format!("{}{}/{}", namespace, o.name, o.version)
                })
            })
            .collect::<Vec<String>>()
    }

    fn list_by_namespace(&self, namespace: &str) -> Vec<&OverlayFile> {
        todo!()
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_registry() {
        let registry = OverlayLocalRegistry::from_dir("core_overlays").unwrap();
        assert_eq!(registry.list_all().len(), 1);
        assert_eq!(registry.get_by_filename("semantic").is_some(), true);
        assert_eq!(registry.get_by_fqn("label/2.0.0").unwrap().unwrap().name, "label");

        // TODO file can include more then one overlay
        let semantic_overlay_file = registry.get_by_filename("semantic").unwrap();
        assert_eq!(semantic_overlay_file.overlays_def.len(), 9);
        let label_overlay = semantic_overlay_file.overlays_def.get(0).unwrap();
        assert_eq!(label_overlay.name, "label");

    }
}
