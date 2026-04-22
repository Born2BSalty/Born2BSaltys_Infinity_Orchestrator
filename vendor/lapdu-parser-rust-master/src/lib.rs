#[path = "main.rs"]
pub mod parser_impl;

use std::path::Path;

pub fn parse_path_to_json(root_path: &Path, preferred_lang: Option<&str>) -> Result<String, String> {
    parser_impl::parse_path_to_json(root_path, preferred_lang)
}
