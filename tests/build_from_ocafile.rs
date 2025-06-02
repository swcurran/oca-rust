#[cfg(test)]
mod test {
    use oca_store::{
        data_storage::{DataStorage, InMemoryDataStorage},
        facade::build::{Error, ValidationError},
        repositories::SQLiteConfig,
        Facade,
    };
    use overlay_file::overlay_registry::OverlayLocalRegistry;

    #[test]
    fn build_from_base() -> Result<(), Error> {
        let db = InMemoryDataStorage::new();
        let db_cache = InMemoryDataStorage::new();
        let cache_storage_config = SQLiteConfig::build().unwrap();
        let ocafile = r#"
ADD ATTRIBUTE d=Text i = Text passed=Boolean
ADD Overlay META
  language="en"
  name = "Entrance credential"
  description = "Entrance credential"
ADD Overlay CHARACTER_ENCODING
   d="utf-8"
   i="utf-8"
   passed="utf-8"
ADD Overlay CONFORMANCE
    d="M"
    i="M"
    passed="M"
ADD Overlay LABEL
 language="en"
 d="Schema digest"
 i="Credential Issuee"
 passed="Passed"
"#
        .to_string();
        let mut facade = Facade::new(Box::new(db), Box::new(db_cache), cache_storage_config);

        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/").unwrap();
        let oca_bundle_model = facade.build_from_ocafile(ocafile, registry)?;

        assert_eq!(
            oca_bundle_model.digest.clone().unwrap().to_string(),
            "EK6E0o2evY4xSpGAO2T10bU0atMBaT9HeCzxUes-JIAv"
        );

        assert_eq!(oca_bundle_model.version, "OCAS02JSON000426_");

        let search_result = facade.search_oca_bundle(None, "Ent".to_string(), 10, 1);
        assert_eq!(search_result.metadata.total, 1);
        Ok(())
    }

    #[test]
    fn build_from_other_bundle() -> Result<(), Error> {
        let _ = env_logger::builder().is_test(true).try_init();
        let db = InMemoryDataStorage::new();
        let db_cache = InMemoryDataStorage::new();
        let cache_storage_config = SQLiteConfig::build().unwrap();
        let mut facade = Facade::new(Box::new(db), Box::new(db_cache), cache_storage_config);
        let other_ocafile = r#"
ADD ATTRIBUTE d=Text i=Text passed=Boolean
ADD OVERLAY META
  language="en"
  name="Entrance credential"
  description="Entrance credential"
ADD OVERLAY CHARACTER_ENCODING
  d="utf-8"
  i="utf-8"
  passed="utf-8"
ADD OVERLAY CONFORMANCE
  d="M"
  i="M"
  passed="M"
ADD OVERLAY LABEL
  language="en"
  d="Schema digest"
  i="Credential Issuee" passed="Passed"
"#
        .to_string();
        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/").unwrap();
        facade.build_from_ocafile(other_ocafile, registry.clone())?;

        let ocafile = r#"
FROM EK6E0o2evY4xSpGAO2T10bU0atMBaT9HeCzxUes-JIAv
ADD ATTRIBUTE x=Text
"#
        .to_string();
        let result = facade.build_from_ocafile(ocafile, registry)?;

        assert_eq!(
            result.digest.unwrap().to_string(),
            "EE7764ppESbfBMAY5S_2yq-wrN1029B0W--j7xIHx1B-"
        );
        Ok(())
    }

    #[test]
    fn build_with_references() -> Result<(), Error> {
        let db = InMemoryDataStorage::new();
        let db_cache = InMemoryDataStorage::new();
        let cache_storage_config = SQLiteConfig::build().unwrap();
        let mut facade = Facade::new(Box::new(db), Box::new(db_cache), cache_storage_config);
        let second_ocafile = r#"
-- name=first
ADD ATTRIBUTE b=Text
"#
        .to_string();

        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/").unwrap();
        facade.build_from_ocafile(second_ocafile, registry.clone())?;

        let third_ocafile = r#"
-- name=second
ADD ATTRIBUTE c=Text
"#
        .to_string();

        facade.build_from_ocafile(third_ocafile, registry.clone())?;

        let ocafile = r#"
ADD ATTRIBUTE A=refs:EMLVHOLXBCiSJ4mXbpFWzKyyxy0a59flf-P-Ok0Fv0ZC
ADD ATTRIBUTE B=refn:first
ADD ATTRIBUTE C=[refn:second]
"#
        .to_string();
        let result = facade.build_from_ocafile(ocafile, registry.clone())?;

        assert_eq!(
            result.digest.unwrap().to_string(),
            "EKnpyX8mk8nRGKS9PDXAWH4bfwA_xWdBlSEXFNNzvH66"
        );

        let from_ocafile = r#"
FROM EKnpyX8mk8nRGKS9PDXAWH4bfwA_xWdBlSEXFNNzvH66
ADD ATTRIBUTE x=Text
"#
        .to_string();

        let result = facade.build_from_ocafile(from_ocafile, registry.clone())?;
        assert_eq!(
            result.digest.unwrap().to_string(),
            "EIQL6cd0lyyc78Hu5RKcwjig4pvR72JU-jx4riQiYrpn"
        );
        let refs = facade.fetch_all_refs().unwrap();

        assert_eq!(refs.len(), 2);
        assert_eq!(
            refs.get("second").unwrap(),
            "EBQDQa2GSKY9AnuyTNt_g8UBzBsOPWKb8Q1w8Zh7_1jX"
        );

        Ok(())
    }

    #[test]
    #[ignore]
    //TODO: move to community overlay
    fn build_with_link() -> Result<(), Error> {
        let db = InMemoryDataStorage::new();
        let db_cache = InMemoryDataStorage::new();
        let cache_storage_config = SQLiteConfig::build().unwrap();
        let mut facade = Facade::new(Box::new(db), Box::new(db_cache), cache_storage_config);
        let first_ocafile = r#"
-- name=first
ADD ATTRIBUTE a=Text
"#
        .to_string();
        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/").unwrap();
        facade.build_from_ocafile(first_ocafile, registry.clone())?;

        let second_ocafile = r#"
-- name=second
ADD ATTRIBUTE b=Text
ADD LINK refn:first ATTRS b=a
"#
        .to_string();

        let result = facade.build_from_ocafile(second_ocafile, registry)?;

        assert_eq!(
            result.digest.unwrap().to_string(),
            "EOUgyR4Pk8Ckz3h5dA-0yFGhwKeNO2z8_PnmymTGMNdi"
        );

        Ok(())
    }

    #[test]
    fn fail_while_building_from_unknown_reference() {
        let db = InMemoryDataStorage::new();
        let db_cache = InMemoryDataStorage::new();
        let cache_storage_config = SQLiteConfig::build().unwrap();
        let mut facade = Facade::new(Box::new(db), Box::new(db_cache), cache_storage_config);

        let ocafile = r#"
ADD ATTRIBUTE A=refs:EI_5ohTYptgOrXldUfZujgd7vcXK9zwa6aNqk4-UDWzq
ADD ATTRIBUTE B=refn:second
ADD ATTRIBUTE C=[refn:third]
"#
        .to_string();
        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/").unwrap();
        let result = facade.build_from_ocafile(ocafile, registry);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, Error::ValidationError(_)));
        if let Error::ValidationError(validation_errors) = error {
            let validation_error = validation_errors.first().unwrap();
            assert!(matches!(validation_error, ValidationError::UnknownRefn(_)));
        }
    }
}
