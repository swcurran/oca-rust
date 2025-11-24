use oca_bundle::state::oca_bundle::OCABundleModel;

struct OCABundleDTO {
    bundle: OCABundleModel,
}

#[allow(dead_code)]
impl OCABundleDTO {
    fn new(bundle: OCABundleModel) -> Self {
        Self { bundle }
    }
}

impl From<OCABundleDTO> for Vec<u8> {
    fn from(val: OCABundleDTO) -> Self {
        let mut digests: Vec<u8> = Vec::new();
        if let Some(ref said) = val.bundle.capture_base.digest {
            digests.push(said.to_string().len().try_into().unwrap());
            digests.extend(said.to_string().as_bytes());
        }

        val.bundle.overlays.iter().for_each(|overlay| {
            if let Some(ref said) = overlay.digest {
                digests.push(said.to_string().len().try_into().unwrap());
                // digests.push(overlay.overlay_type().into());
                digests.extend(said.to_string().as_bytes());
            }
        });

        digests
    }
}

#[cfg(test)]
mod tests {
    use overlay_file::overlay_registry::OverlayLocalRegistry;

    use super::*;

    #[test]
    #[ignore]
    fn test_to_digests() {
        // TODO seems this object is wrong
        let oca_str = r#"
{
  "v": "OCAS02JSON0007c1_",
  "digest": "EDTaoqiaaL50dP-HTxYWuiniwhrzGcP9ji-mPeJgudLk",
  "capture_base": {
    "digest": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
    "type": "capture_base/2.0.0",
    "attributes": {
      "d": "Text",
      "el": "Text",
      "i": "Text",
      "list": [
        "Text"
      ],
      "passed": "Boolean"
    }
  },
  "overlays": [
    {
      "digest": "EKN9PGIHxLuZe92ZDyrZulScFgTfAdjEc9xXEVb_WULX",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "Meta/2.0.0",
      "description": "Entrance credential",
      "name": "Entrance credential"
    },
    {
      "digest": "EFOAxxDMSnOiuah9OwoCdwkns8EfsurcHXF57-XdGnen",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "Character_Encoding/2.0.0",
      "d": "utf-8",
      "i": "utf-8",
      "passed": "utf-8"
    },
    {
      "digest": "ELivUa6QlCOpidnqLDs9Il1uqILb9pBUj2rLdGgqWDwv",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "conformance/2.0.0",
      "d": "M",
      "i": "M",
      "passed": "M"
    },
    {
      "digest": "ECsW-Zb7A0TfG_M_HNH9wwKqil3rSiyKEfPE4398aQdC",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "label/2.0.0",
      "d": "Schema digest",
      "i": "Credential Issuee",
      "passed": "Passed"
    },
    {
      "digest": "EJcEfNE3s_lZeUF1C_tez3qbThSsIJq4qV6WHlo-hmIL",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "format/2.0.0",
      "d": "image/jpeg"
    },
    {
      "digest": "EICOF_bxwUyKC7W-blp51-YPPieJxqDPL7wrSkeT8jOg",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "unit/2.0.0",
      "i": "m"
    },
    {
      "digest": "EPT1EDp2ofO1xJSQFehyZb8kCfMqXV8giTs0MeqQOp2a",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "cardinality/2.0.0",
      "list": "1-2"
    },
    {
      "digest": "EKJ1z6PIFXqfP7wy6Hj21Of23HcoiT-b5P1qs_DgYJHo",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "ENTRY_CODE/2.0.0",
      "el": [
        "o1",
        "o2",
        "o3"
      ],
      "list": "entry_code_said"
    },
    {
      "digest": "EKohdNuyxHWPZ1dy-Om5Rx4RxufHM5jjDKBa3jyRvp52",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "ENTRY/2.0.0",
      "el": {
        "o1": "o1_label",
        "o2": "o2_label",
        "o3": "o3_label"
      },
      "list": "refs:ENrf7niTCnz7HD-Ci88rlxHlxkpQ2NIZNNv08fQnXANI"
    }
  ]
}
"#;
        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/").unwrap();
        let oca = oca_bundle::controller::load_oca(&mut oca_str.as_bytes(), &registry).unwrap();
        let digests: Vec<u8> = OCABundleDTO::new(oca).into();
        assert_eq!(
            digests,
            vec![
                44, 69, 73, 74, 71, 74, 109, 83, 95, 80, 57, 106, 119, 90, 68, 97, 109, 66, 54, 99,
                84, 71, 57, 77, 111, 88, 75, 82, 117, 50, 49, 109, 121, 106, 88, 115, 77, 105, 55,
                71, 89, 100, 100, 68, 121
            ]
        )
    }
}
