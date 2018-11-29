use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use regex::Regex;
use tempfile::tempdir;

use config::Config;
use query;
use vcs::Vcs;

pub struct Sync {
    vcs: Vcs,
    languages: HashMap<String, PathBuf>,
    source_language: String,
    source_path: String,
    export_dir: String,
}

fn find_languages(
    source_dir: &PathBuf, 
    match_regex: &str, 
    lang_regex: &str) 
    -> Result<HashMap<String, PathBuf>, Box<Error>> {
    let match_regex = Regex::new(match_regex)?;
    let lang_regex = Regex::new(lang_regex)?;
    let mut languages: HashMap<String, PathBuf> = HashMap::new();

    for entry in fs::read_dir(&source_dir)? {
            let path = entry.unwrap().path();
            let file_name = path.file_name().unwrap();
            let file_name_as_str = file_name.to_str().unwrap();

            if match_regex.is_match(file_name_as_str) {
                let lang = lang_regex.captures(file_name_as_str);

                if lang.is_some() {
                    languages.insert(lang.unwrap()[1].to_string(), path.to_path_buf());
                }
            }
    }

    Ok(languages)
}

impl Sync {
    pub fn new(config: &Config) -> Result<Self, Box<Error>> {
         let source_dir = Path::new(&config.vcs.local_path)
            .join(config.job.source_dir.to_string());

        if !source_dir.is_dir() {
            Err(format!("source_dir {:?} does not exist or not a directory", &source_dir))?;
        }

        let languages = find_languages(&source_dir, &config.job.source_match, &config.job.source_name_match)?;

        if languages.is_empty() {
            Err(format!("source_dir {:?} doesn't contain any languages", &source_dir))?;
        }
        
        if !languages.contains_key(&config.job.source_language) {
            Err(format!("source_dir {:?} doesn't contain source language {}", 
                &source_dir, &config.job.source_language))?;
        }

        let vcs = Vcs::new(config)?;
        vcs.pull()?;

        let sync = Sync {
            vcs: vcs,
            languages: languages,
            source_language: config.job.source_language.to_string(),
            source_path: config.job.source_dir.to_string(),
            export_dir: config.job.export_dir.to_string(),
        };

        Ok(sync)
    }

    fn vcs_relative_path(&self, file_name: &OsStr) -> PathBuf {
        Path::new(&self.source_path).join(file_name)
    }

    pub fn source_language_path(&self) -> &PathBuf {
        self.languages.get(&self.source_language).unwrap()
    } 

    pub fn push(&self) -> Result<(), Box<Error>> {
        unimplemented!();
    }

    pub fn pull(
        &self, 
        old_commit: Option<&str>, 
        new_commit: Option<&str>) 
        -> Result<(), Box<Error>> {
        let source_language_path = self.source_language_path();
        let source_language_filename = source_language_path.file_name().unwrap();
        let temp_dir = tempdir()?;
        let source_path = self.vcs_relative_path(source_language_filename);
        let out_path = Path::new(&self.export_dir).join(source_language_filename);

        if old_commit.is_none() {
            // Grab the HEAD source language and use it as our source
            self.vcs.show(&source_path.to_string_lossy(), None, &out_path.to_string_lossy())?;
        } else {
            // Grab the old and new commits, query the changes, and use the output as our source
            let old_filename = format!("{}.{}", 
                old_commit.unwrap(), 
                source_language_filename.to_string_lossy());
            let new_filename = format!("{}.{}",
                if new_commit.is_some() { new_commit.unwrap() } else { "HEAD"},
                source_language_filename.to_string_lossy());

            let old_path = Path::new(temp_dir.path()).join(old_filename);
            let new_path = Path::new(temp_dir.path()).join(new_filename);

            self.vcs.show(&source_path.to_string_lossy(), old_commit, &old_path.to_string_lossy())?;
            self.vcs.show(&source_path.to_string_lossy(), new_commit, &new_path.to_string_lossy())?;

            // TODO: is the the correct comparison order?
            let query = query::query_change(&old_path.to_string_lossy(), &new_path.to_string_lossy());
            query.run(Some(&out_path.to_string_lossy()));
        }

        for (language, path) in &self.languages {
            if language == &self.source_language {
                continue
            }

            let file_name = format!("{}.{}",
                if new_commit.is_some() { new_commit.unwrap() } else { "HEAD"},
                path.file_name().unwrap().to_string_lossy());
            let file_path = Path::new(&temp_dir.path()).join(file_name);

            let target_language_filename = path.file_name().unwrap();
            let target_path = self.vcs_relative_path(target_language_filename);
            self.vcs.show(&target_path.to_string_lossy(), new_commit, &file_path.to_string_lossy())?;

            // TODO panic if export_dir doesn't exits
            let target_out_path = Path::new(&self.export_dir).join(target_language_filename);

            let query = query::query_diff(&out_path.to_string_lossy(), &target_path.to_string_lossy());
            query.run(Some(&target_out_path.to_string_lossy()));
        }

        Ok(())
    }
}