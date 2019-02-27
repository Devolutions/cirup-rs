use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::path::PathBuf;

pub struct RevisionRange {
    pub old_rev: Option<String>,
    pub new_rev: Option<String>
}

impl Default for RevisionRange {
    fn default() -> RevisionRange {
        RevisionRange {
            old_rev: None,
            new_rev: None,
        }
    }
}

impl PartialEq for RevisionRange {
    fn eq(&self, other: &RevisionRange) -> bool {
       self.old_rev == other.old_rev && self.new_rev == other.new_rev
    }
}

impl Eq for RevisionRange { }

impl fmt::Display for RevisionRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl RevisionRange {
    pub fn new<S: Into<String>>(old_rev: Option<S>, new_rev: Option<S>) -> Self {
        RevisionRange {
            old_rev: old_rev.map(|s| s.into()),
            new_rev: new_rev.map(|s| s.into()),
        }
    }

    pub fn old_rev_as_ref(&self) -> Option<&str> {
        self.old_rev.as_ref().map(String::as_str)
    }

    pub fn new_rev_as_ref(&self) -> Option<&str> {
        self.new_rev.as_ref().map(String::as_str)
    }

    fn to_string(&self) -> String {
        format!("{}{}", self.old_rev.as_ref().unwrap_or(&String::default()), 
            format!("{}{}", 
            if self.old_rev.is_some() && self.new_rev.is_some() { let x = "-"; x } else { "" },
            self.new_rev.as_ref().unwrap_or(&String::default())))
    }

    fn from_string(string: &str) -> RevisionRange {
        let mut revision_range = RevisionRange::default();
        let split = string.split("-")
            .take(2)
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>();

        if let Some(y) = split.get(1) {
            revision_range.new_rev = Some(y.to_string());

            if let Some(x) = split.get(0) {
                revision_range.old_rev = Some(x.to_string());
            }
        }
        else if let Some(x) = split.get(0) {
            revision_range.new_rev = Some(x.to_string());
        }

        revision_range
    }

    pub fn append_to_file_name(&self, path: PathBuf) -> Result<PathBuf, Box<Error>> {
        let rev = format!("~{}~", self.to_string());
        let file_stem = path.file_stem().unwrap_or(OsStr::new(""));
        let mut file_name = file_stem.to_os_string();

        if !rev.is_empty() {
            file_name.push(format!(".{}", rev));
        }

        if let Some(extension) = path.extension() {
            file_name.push(".");
            file_name.push(extension)
        }

        Ok(path.with_file_name(file_name))
    }

    pub fn extract_from_file_name(path: PathBuf) -> (RevisionRange, PathBuf) {
        let mut revision_range = RevisionRange { old_rev: None, new_rev: None };
        let file_stem = path.file_stem().unwrap_or(OsStr::new(""));
        let file_name = file_stem.to_string_lossy();
        let mut split = file_name.split(".")
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>();

        if split.len() > 1 {
            let revision = split.pop().unwrap();

            if revision.starts_with('~') && revision.ends_with('~') {
                let trimmed = revision.trim_matches('~');
                revision_range = RevisionRange::from_string(trimmed);
            } else {
                split.push(revision);
            }
        }

        let mut file_name = split.join(".").to_string();

        if let Some(extension) = path.extension() {
            file_name.push_str(".");
            file_name.push_str(&extension.to_string_lossy());
        }

        (revision_range, path.with_file_name(file_name))
    }
}


#[test]
fn revision_range_to_string_test() {
    let mut a = RevisionRange { old_rev: Some("r123".to_string()), new_rev: Some("r456".to_string()) };
    assert_eq!(a.to_string(), "r123-r456");
    a = RevisionRange { old_rev: Some("r123".to_string()), new_rev: None };
    assert_eq!(a.to_string(), "r123");
    a = RevisionRange { old_rev: Some("r456".to_string()), new_rev: None };
    assert_eq!(a.to_string(), "r456");
    a = RevisionRange { old_rev: None, new_rev: None };
    assert_eq!(a.to_string(), "");
}

#[test]
fn revision_range_from_string_test() {
    let mut a = RevisionRange::from_string("r123-r456");
    assert_eq!(a.old_rev, Some("r123".to_string()));
    assert_eq!(a.new_rev, Some("r456".to_string()));
    a = RevisionRange::from_string("r123");
    assert_eq!(a.old_rev, None);
    assert_eq!(a.new_rev, Some("r123".to_string()));
    a = RevisionRange::from_string("");
    assert_eq!(a.old_rev, None);
    assert_eq!(a.new_rev, None);
    a = RevisionRange::from_string("-");
    assert_eq!(a.old_rev, None);
    assert_eq!(a.new_rev, None);
}

#[test]
fn revision_range_append_to_file_name_test() {
    let mut p = PathBuf::from("/test/path/myfile.resx");
    let rev = RevisionRange { old_rev: Some("r123".to_string()), new_rev: Some("r456".to_string()) };
    p = rev.append_to_file_name(p).unwrap();
    assert_eq!(p, PathBuf::from("/test/path/myfile.~r123-r456~.resx"));
}

    #[test]

fn revision_range_extract_from_file_name_test() {
    let (revision, path) = RevisionRange::extract_from_file_name(PathBuf::from("/test/path/myfile.~r123-r456~.resx"));
    assert_eq!(revision.old_rev, Some("r123".to_string()));
    assert_eq!(revision.new_rev, Some("r456".to_string()));
    assert_eq!(path, PathBuf::from("/test/path/myfile.resx"));
    let (revision, path) = RevisionRange::extract_from_file_name(PathBuf::from("/test/path/myfile.not.a.revision.resx"));
    assert_eq!(revision.old_rev, None);
    assert_eq!(revision.new_rev, None);
    assert_eq!(path, PathBuf::from("/test/path/myfile.not.a.revision.resx"));
}