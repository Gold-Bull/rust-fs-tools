use clap::builder::ValueParser;
use std::{fs, path::PathBuf};

pub fn check_if_parent_path_exists() -> ValueParser {
    ValueParser::from(move |s: &str| -> std::result::Result<PathBuf, String> {
        let parent_path = std::path::Path::new(s).parent().unwrap();
        if fs::metadata(parent_path)
            .map_err(|e| e.to_string())?
            .is_dir()
        {
            Ok(PathBuf::from(s))
        } else {
            Err(format!("Unable to access path '{}'", s))
        }
    })
}

pub fn check_if_directory_exists() -> ValueParser {
    ValueParser::from(move |s: &str| -> std::result::Result<PathBuf, String> {
        if fs::metadata(s).map_err(|e| e.to_string())?.is_dir() {
            Ok(PathBuf::from(s))
        } else {
            Err(format!("Unable to access path '{}'", s))
        }
    })
}

pub fn check_if_file_exists() -> ValueParser {
    ValueParser::from(move |s: &str| -> std::result::Result<PathBuf, String> {
        if fs::metadata(s).map_err(|e| e.to_string())?.is_file() {
            Ok(PathBuf::from(s))
        } else {
            Err(format!("Unable to access path '{}'", s))
        }
    })
}
