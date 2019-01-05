use std::boxed::Box;
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
use vcs;
use vcs::Vcs;

pub struct Sync {
    vcs: Box<Vcs>,
    languages: HashMap<String, PathBuf>,
    source_language: String,
    source_path: String,
    working_dir: String,
    match_rex: Regex,
    lang_rex: Regex,
}

fn find_languages(
    source_dir: &PathBuf, 
    match_regex: &Regex, 
    lang_regex: &Regex) 
    -> Result<HashMap<String, PathBuf>, Box<Error>> {
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
        let vcs = vcs::new(config)?;
        vcs.pull()?;

         let source_dir = Path::new(&config.vcs.local_path)
            .join(config.sync.source_dir.to_string());

        if !source_dir.is_dir() {
            Err(format!("source_dir {:?} does not exist or not a directory", &source_dir))?;
        }

        let working_dir = Path::new(&config.sync.working_dir);

        if !working_dir.is_dir() {
            fs::create_dir_all(working_dir)?;
        }

        let match_rex = Regex::new(&config.sync.source_match)?;
        let lang_rex = Regex::new(&config.sync.source_name_match)?;

        let languages = find_languages(&source_dir, &match_rex, &lang_rex)?;

        if languages.is_empty() {
            Err(format!("source_dir {:?} doesn't contain any languages", &source_dir))?;
        }
        
        if !languages.contains_key(&config.sync.source_language) {
            Err(format!("source_dir {:?} doesn't contain source language {}", 
                &source_dir, &config.sync.source_language))?;
        }

        let sync = Sync {
            vcs: vcs,
            languages: languages,
            source_language: config.sync.source_language.to_string(),
            source_path: config.sync.source_dir.to_string(),
            working_dir: config.sync.working_dir.to_string(),
            match_rex: match_rex,
            lang_rex: lang_rex,
        };

        Ok(sync)
    }

    fn vcs_relative_path(&self, file_name: &OsStr) -> PathBuf {
        Path::new(&self.source_path).join(file_name)
    }

    pub fn source_language_path(&self) -> &PathBuf {
        self.languages.get(&self.source_language).unwrap()
    } 

    pub fn push(&self, force: bool) -> Result<(), Box<Error>> {
        println!("starting push...");

        let source_language_path = self.source_language_path();
        let source_language_filename = source_language_path.file_name().unwrap();
        let source_path = self.vcs_relative_path(source_language_filename);
        let out_path = Path::new(&self.working_dir).join(source_language_filename);

        // Grab the HEAD source language for validation
        self.vcs.show(&source_path.to_string_lossy(), None, &out_path.to_string_lossy())?;

        println!("looking for translations in {}", &self.working_dir);

        let translations = find_languages(&Path::new(&self.working_dir).to_path_buf(), 
            &self.match_rex, &self.lang_rex)?;

        if translations.is_empty() {
            Err(format!("working_dir {:?} doesn't contain any translations", &self.working_dir))?;
        }

        println!("found {} translations", translations.keys().count());

        for (language, path) in &translations {
            println!("processing translation: {}", language);

            if language != &self.source_language {
                let query_string = r"
                    SELECT
                        A.key, A.val
                    FROM A
                    INNER JOIN B on (A.key = B.key) and (A.val = B.val)";
                let query = query::CirupQuery::new(query_string, &source_language_path.to_string_lossy(), 
                    Some(&path.to_string_lossy()));

                if !query.run().is_empty() && !force {
                    Err(format!(r"translation {} contains untranslated strings. 
                    translate all strings or use use the force option.", language))?;
                }
            }

            match self.languages.get(language) {
                Some(vcs_language_path) => {
                    println!("merging {:?} into {:?}", path, vcs_language_path);
                    let query = query::query_merge(&vcs_language_path.to_string_lossy(), &path.to_string_lossy());
                    query.run_interactive(Some(&vcs_language_path.to_string_lossy()));
                },
                None => {
                    println!("no source language for translation {}", language);
                },
            }
        }

        println!("push complete");

        Ok(())
    }

/*
If no old commit is specified:
    diff the source language with the other languages
    generate an output file for every target language, with the missing translations
If an old commit is specified:
    diff the changes between the old and new version of the source language (new and updated strings)
    generate an output file for every target language, with missing missing translations, 
    and translations (potentially) needing an update
*/
    pub fn pull(
        &self, 
        old_commit: Option<&str>, 
        new_commit: Option<&str>) 
        -> Result<(), Box<Error>> {
        println!("starting pull...");

        let source_language_path = self.source_language_path();
        let source_language_filename = source_language_path.file_name().unwrap();
        let temp_dir = tempdir()?;
        let source_path = self.vcs_relative_path(source_language_filename);
        let out_path = Path::new(&self.working_dir).join(source_language_filename);

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

            let query = query::query_change(&new_path.to_string_lossy(), &old_path.to_string_lossy());
            query.run_interactive(Some(&out_path.to_string_lossy()));
        }

        for (language, path) in &self.languages {
            if language == &self.source_language {
                continue
            }

            println!("processing translation: {}", language);

            let file_name = format!("{}.{}",
                if new_commit.is_some() { new_commit.unwrap() } else { "HEAD"},
                path.file_name().unwrap().to_string_lossy());
            let file_path = Path::new(&temp_dir.path()).join(file_name);

            let target_language_filename = path.file_name().unwrap();
            let target_path = self.vcs_relative_path(target_language_filename);
            self.vcs.show(&target_path.to_string_lossy(), new_commit, &file_path.to_string_lossy())?;

            let target_out_path = Path::new(&self.working_dir).join(target_language_filename);

            let query : query::CirupQuery;
            
            if old_commit.is_none() {
                query = query::query_diff(&out_path.to_string_lossy(), &file_path.to_string_lossy());
            } else {
                let query_string = r"
                    SELECT
                        A.key,
                        (CASE WHEN B.val IS NOT NULL
                         THEN B.val
                         ELSE A.val END)
                    FROM A
                    LEFT OUTER JOIN B on A.key = B.key";
                query = query::CirupQuery::new(query_string, &out_path.to_string_lossy(), Some(&file_path.to_string_lossy()));
            }
            
            println!("creating translation in {:?}", target_out_path);

            query.run_interactive(Some(&target_out_path.to_string_lossy()));
        }

        println!("pull complete");

        Ok(())
    }
}