use ::config;

use regex::Regex;

use std::path::Path;
use std::fs;

pub struct Job {
    pub source_language: String,
    pub languages: Vec<String>,
}

impl Job {
    pub fn new(config: &config::Config) -> Job {
        let source_dir = Path::new(&config.vcs.local_path)
            .join(config.job.source_dir.to_string());

        if !source_dir.is_dir() {
            panic!("path {:?} is not a directory", source_dir);
        }

        let re = Regex::new(&config.job.source_match).unwrap();
        let fe_re = Regex::new(&config.job.source_name_match).unwrap();

        let mut source_language : Option<String> = None;
        let mut languages = Vec::<String>::new();

        for entry in fs::read_dir(&source_dir)
            .expect("error traversing directory") {
            let entry = entry.unwrap();
            let entry_path = entry.path();
            let file_name = entry_path.file_name().unwrap();
            let file_name_as_str = file_name.to_str().unwrap();

            if re.is_match(file_name_as_str) {
                let fe = fe_re.captures(file_name_as_str).unwrap();

                if &fe[1] == &config.job.source_language {
                    source_language = Some(entry_path.to_string_lossy().to_string());
                } else {
                    languages.push(entry_path.to_string_lossy().to_string());
                }
            }
        }

        Job { 
            source_language: source_language.unwrap(),
            languages: languages,
        }
    }
}