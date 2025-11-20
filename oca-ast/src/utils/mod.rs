use isolang::Language;

pub fn parse_language_code(code: &str) -> Result<Language, String> {
    // Try ISO 639-1 first
    if let Some(lang) = Language::from_639_1(code) {
        return Ok(lang);
    }

    // Try ISO 639-3
    if let Some(lang) = Language::from_639_3(code) {
        return Ok(lang);
    }

    // Handle language code with country code (e.g., "en_UK" or "en-UK")
    let separator = if code.contains('_') { '_' } else { '-' };
    if let Some((lang_code, _country_code)) = code.split_once(separator)
        && let Some(lang) = Language::from_639_1(lang_code)
    {
        return Ok(lang);
    }

    Err(format!("Invalid language code: {}", code))
}

pub fn is_valid_language_code(code: &str) -> bool {
    parse_language_code(code).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso_639_1() {
        assert!(parse_language_code("en").is_ok());
        assert!(parse_language_code("pl").is_ok());
    }

    #[test]
    fn test_parse_iso_639_3() {
        assert!(parse_language_code("eng").is_ok());
        assert!(parse_language_code("pol").is_ok());
    }

    #[test]
    fn test_parse_with_country_code() {
        assert!(parse_language_code("en_US").is_ok());
        assert!(parse_language_code("en-UK").is_ok());
        assert!(parse_language_code("pl_PL").is_ok());
    }

    #[test]
    fn test_invalid_code() {
        assert!(parse_language_code("invalid").is_err());
        assert!(parse_language_code("xx_YY").is_err());
    }
}
