#[path = "main.rs"]
pub mod parser_impl;

use std::path::Path;

pub fn parse_path_to_json(root_path: &Path, preferred_lang: Option<&str>) -> Result<String, String> {
    parser_impl::parse_path_to_json(root_path, preferred_lang)
}

pub fn parse_path_to_json_with_tra(
    root_path: &Path,
    preferred_lang: Option<&str>,
    inherited_tra: &std::collections::HashMap<String, String>,
) -> Result<String, String> {
    parser_impl::parse_path_to_json_with_tra(root_path, preferred_lang, inherited_tra)
}
