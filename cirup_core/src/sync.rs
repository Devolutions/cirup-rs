use std::boxed::Box;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use regex::Regex;
use tempfile;

use config::Config;
use query;
use revision::RevisionRange;
use utils::*;
use vcs;
use vcs::Vcs;

pub struct Sync {
    pub vcs: Box<Vcs>,
    languages: HashMap<String, LanguageFile>,
    source_language: String,
    source_path: String,
    working_dir: String,
    match_rex: Regex,
    lang_rex: Regex,
    temp_dir: tempfile::TempDir,
}

struct LanguageFile {
    path: PathBuf,
    file_name: String,
    _file_ext: String,
    revision: RevisionRange,
}

impl Default for LanguageFile {
    fn default() -> LanguageFile {
        LanguageFile {
            path: PathBuf::default(),
            file_name: String::new(),
            _file_ext: String::new(),
            revision: RevisionRange::default(),
        }
    }
}

impl LanguageFile {
    fn load<T: AsRef<Path>>(path: T) -> Result<LanguageFile, Box<Error>> {
        let path_ref = PathBuf::from(path.as_ref());
        if !path_ref.is_file() {
            Err("invalid language file")?;
        };

        let file_ext = match path_ref.extension().and_then(OsStr::to_str) {
            Some(extension) => extension.to_string(),
            _ => Err(format!("invalid language file {:?}", path_ref))?,
        };
        let (language_revision, path) =
            RevisionRange::extract_from_file_name(PathBuf::from(path.as_ref()));
        let file_name = match path.file_name().and_then(OsStr::to_str) {
            Some(file_name) => file_name.to_string(),
            _ => Err(format!("invalid language file {:?}", path_ref))?,
        };

        Ok(LanguageFile {
            path: path_ref,
            file_name: file_name,
            _file_ext: file_ext,
            revision: language_revision,
        })
    }
}

fn find_languages(
    source_dir: &PathBuf,
    match_regex: &Regex,
    lang_regex: &Regex,
) -> Result<HashMap<String, LanguageFile>, Box<Error>> {
    let mut languages: HashMap<String, LanguageFile> = HashMap::new();

    for entry in fs::read_dir(&source_dir)? {
        if let Ok(language_file) = LanguageFile::load(entry.unwrap().path()) {
            if !match_regex.is_match(&language_file.file_name) {
                continue;
            }

            if let Some(captures) = lang_regex.captures(&language_file.file_name.to_string()) {
                languages.insert(captures[1].to_string(), language_file);
            }
        }
    }

    Ok(languages)
}

impl Sync {
    pub fn new(config: &Config) -> Result<Self, Box<Error>> {
        let vcs = vcs::new(config)?;
        vcs.pull()?;

        let source_dir = Path::new(&config.vcs.local_path).join(config.sync.source_dir.to_string());

        if !source_dir.is_dir() {
            Err(format!(
                "source_dir {:?} does not exist or not a directory",
                &source_dir
            ))?;
        }

        let working_dir = Path::new(&config.sync.working_dir);

        if !working_dir.is_dir() {
            fs::create_dir_all(working_dir)?;
        }

        let match_rex = Regex::new(&config.sync.match_language_file)?;
        let lang_rex = Regex::new(&config.sync.match_language_name)?;

        let mut languages = find_languages(&source_dir, &match_rex, &lang_rex)?;

        if languages.is_empty() {
            Err(format!(
                "couldn't find any language files in {:?}",
                &source_dir
            ))?;
        }

        languages.retain(|key, _value| {
            key == &config.sync.source_language || config.sync.target_languages.contains(key)
        });

        if !languages.contains_key(&config.sync.source_language) {
            Err(format!(
                "couldn't find source language file in {:?}",
                &source_dir
            ))?;
        }

        let sync = Sync {
            vcs: vcs,
            languages: languages,
            source_language: config.sync.source_language.to_string(),
            source_path: config.sync.source_dir.to_string(),
            working_dir: config.sync.working_dir.to_string(),
            match_rex: match_rex,
            lang_rex: lang_rex,
            temp_dir: tempfile::tempdir()?,
        };

        Ok(sync)
    }

    fn vcs_relative_path<P: AsRef<Path>>(&self, file_name: P) -> PathBuf {
        Path::new(&self.source_path).join(file_name)
    }

    fn source_language(&self) -> Option<&LanguageFile> {
        self.languages.get(&self.source_language)
    }

    pub fn source_language_path(&self) -> PathBuf {
        if let Some(language) = self.source_language() {
            return language.path.to_owned();
        }

        PathBuf::default()
    }

    fn create_source_language_file(
        &self,
        rev: &RevisionRange,
        show_changes: bool,
    ) -> Result<PathBuf, Box<Error>> {
        debug!("preparing source file for revision(s) {}", rev);
        let source = self.source_language().unwrap();
        let source_path_vcs = self.vcs_relative_path(&source.file_name);
        let source_path_out =
            rev.append_to_file_name(Path::new(&self.working_dir).join(&source.file_name))?;

        let old_commit = rev.old_rev_as_ref();
        let new_commit = rev.old_rev_as_ref();

        if rev.old_rev.is_none() {
            self.vcs.show(
                &source_path_vcs.to_string_lossy(),
                None,
                &source_path_out.to_string_lossy(),
            )?;
        } else {
            let rev_old = RevisionRange::new(old_commit, None);
            let old_path = rev_old
                .append_to_file_name(Path::new(self.temp_dir.path()).join(&source.file_name))?;
            self.vcs.show(
                &source_path_vcs.to_string_lossy(),
                old_commit,
                &old_path.to_string_lossy(),
            )?;

            let rev_new = RevisionRange::new(None, new_commit);
            let new_path = rev_new
                .append_to_file_name(Path::new(self.temp_dir.path()).join(&source.file_name))?;
            self.vcs.show(
                &source_path_vcs.to_string_lossy(),
                new_commit,
                &new_path.to_string_lossy(),
            )?;

            debug!(
                "generating source file from {} and {}",
                old_path.display(),
                new_path.display()
            );

            let query: query::CirupQuery;

            if show_changes {
                query =
                    query::query_change(&new_path.to_string_lossy(), &old_path.to_string_lossy());
            } else {
                query = query::query_diff(&new_path.to_string_lossy(), &old_path.to_string_lossy());
            }

            query.run_interactive(Some(&source_path_out.to_string_lossy()));
        }

        debug!("source file path is {}", source_path_out.display());

        Ok(source_path_out)
    }

    pub fn push(
        &self,
        old_commit: Option<&str>,
        new_commit: Option<&str>,
    ) -> Result<(), Box<Error>> {
        let current_rev = sanitized(&self.vcs.current_revision()?).to_string();
        let rev = RevisionRange::new(
            old_commit,
            match new_commit {
                Some(new_commit) => Some(new_commit),
                None => Some(&current_rev),
            },
        );
        let source_path_out = self.create_source_language_file(&rev, true)?;

        let mut translations = find_languages(
            &Path::new(&self.working_dir).to_path_buf(),
            &self.match_rex,
            &self.lang_rex,
        )?;
        translations.retain(|_key, value| value.revision == rev);
        if translations.is_empty() {
            Err(format!(
                "no pending translations for revision(s) {} in {}",
                rev, &self.working_dir
            ))?;
        }

        for (language, language_file) in &translations {
            if language != &self.source_language {
                debug!("preparing to push {}", language);

                let query_string = r"
                    SELECT
                        B.key, B.val
                    FROM B
                    INNER JOIN A on (A.key = B.key) AND (A.val <> B.val)";

                let query = query::CirupQuery::new(
                    query_string,
                    &source_path_out.to_string_lossy(),
                    Some(&language_file.path.to_string_lossy()),
                );
                let file_path = rev.append_to_file_name(
                    Path::new(self.temp_dir.path()).join(&language_file.file_name),
                )?;

                debug!(
                    "generating intermediate file from {} and {}",
                    source_path_out.display(),
                    language_file.path.display()
                );
                query.run_interactive(Some(&file_path.to_string_lossy()));

                match self.languages.get(language) {
                    Some(vcs_language_file) => {
                        debug!(
                            "merging {} into {}",
                            language_file.path.display(),
                            vcs_language_file.path.display()
                        );
                        let query = query::query_merge(
                            &vcs_language_file.path.to_string_lossy(),
                            &file_path.to_string_lossy(),
                        );
                        query.run_interactive(Some(&vcs_language_file.path.to_string_lossy()));
                        info!(
                            "merged translation for {} from {} into {}",
                            language,
                            language_file.path.display(),
                            vcs_language_file.path.display()
                        );
                    }
                    None => {
                        warn!("no source language for {} in version control!", language);
                    }
                }
            }
        }

        Ok(())
    }

    /*
    If no old commit is specified:
        diff the source language with the other languages
        generate an output file for every target language, with the missing translations
    If an old commit is specified:
        new commit defaults to HEAD
        diff the changes between the old and new version of the source language
        generate an output file for every target language, with missing missing translations
        (and, optionally, changed translations)
    */
    pub fn pull(
        &self,
        old_commit: Option<&str>,
        new_commit: Option<&str>,
        show_changes: bool,
    ) -> Result<(), Box<Error>> {
        let current_rev = sanitized(&self.vcs.current_revision()?);
        let rev = RevisionRange::new(
            old_commit,
            match new_commit {
                Some(new_commit) => Some(new_commit),
                None => Some(&current_rev),
            },
        );
        let source_path_out = self.create_source_language_file(&rev, show_changes)?;

        for (language, language_file) in &self.languages {
            if language == &self.source_language {
                continue;
            }

            debug!("generating translation for {}", language);

            let target_path_vcs = self.vcs_relative_path(&language_file.file_name);
            let target_path_out = rev
                .append_to_file_name(Path::new(&self.working_dir).join(&language_file.file_name))?;

            let file_path = RevisionRange::new(None, rev.new_rev_as_ref()).append_to_file_name(
                Path::new(self.temp_dir.path()).join(&language_file.file_name),
            )?;
            self.vcs.show(
                &target_path_vcs.to_string_lossy(),
                new_commit,
                &file_path.to_string_lossy(),
            )?;

            let query: query::CirupQuery;

            if old_commit.is_none() {
                query = query::query_diff(
                    &source_path_out.to_string_lossy(),
                    &file_path.to_string_lossy(),
                );
            } else {
                let query_string = r"
                    SELECT
                        A.key, A.val
                    FROM A
                    LEFT OUTER JOIN B on A.key = B.key";
                query = query::CirupQuery::new(
                    query_string,
                    &source_path_out.to_string_lossy(),
                    Some(&file_path.to_string_lossy()),
                );
            }

            query.run_interactive(Some(&target_path_out.to_string_lossy()));

            info!(
                "generated translation for {} in {}",
                language,
                target_path_out.display()
            );
        }

        Ok(())
    }
}
