use crate::{parse_from_string, OverlayDef};
use std::{collections::HashMap, fs, path::{Path, PathBuf}};
use crate::OverlayFile;

pub trait OverlayRegistry {
    fn get_by_filename(&self, name: &str) -> Option<&OverlayFile>;
    fn get_by_name(&self, namespace: Option<&str>, name: &str,) -> Option<&OverlayDef>;
    fn list_by_namespace(&self, namespace: &str) -> Vec<&OverlayFile>;

    fn list_all(&self) -> Vec<String>;
}

#[derive(Debug)]
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

    fn get_by_name(&self, namespace: Option<&str>, name: &str) -> Option<&OverlayDef> {
        let name = name.to_ascii_lowercase();
        let namespace = namespace.map(|ns| ns.to_ascii_lowercase());

        self.overlays.values().find_map(|overlay_file| {
            overlay_file.overlays_def.iter().find(|o| {
                let o_ns = o.namespace.as_ref().map(|s| s.to_ascii_lowercase());
                o_ns == namespace && o.name.eq_ignore_ascii_case(&name)
            })
        })
    }


    fn list_all(&self) -> Vec<String> {
        self.overlays.keys().cloned().collect()
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
        assert_eq!(registry.get_by_name(None, "label").unwrap().overlays_def.first().unwrap().name, "label");

        // TODO file can include more then one overlay
        let semantic_overlay_file = registry.get_by_filename("semantic").unwrap();
        assert_eq!(semantic_overlay_file.overlays_def.len(), 1);
        let label_overlay = semantic_overlay_file.overlays_def.get(0).unwrap();
        assert_eq!(label_overlay.name, "label");

    }
}
