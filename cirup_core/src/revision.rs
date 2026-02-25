use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct RevisionRange {
    pub(crate) old_rev: Option<String>,
    pub(crate) new_rev: Option<String>,
}

impl PartialEq for RevisionRange {
    fn eq(&self, other: &RevisionRange) -> bool {
        self.old_rev == other.old_rev && self.new_rev == other.new_rev
    }
}

impl Eq for RevisionRange {}

impl fmt::Display for RevisionRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_range())
    }
}

impl RevisionRange {
    pub(crate) fn new<S: Into<String>>(old_rev: Option<S>, new_rev: Option<S>) -> Self {
        RevisionRange {
            old_rev: old_rev.map(|s| s.into()),
            new_rev: new_rev.map(|s| s.into()),
        }
    }

    pub(crate) fn old_rev_as_ref(&self) -> Option<&str> {
        self.old_rev.as_deref()
    }

    pub(crate) fn new_rev_as_ref(&self) -> Option<&str> {
        self.new_rev.as_deref()
    }

    fn format_range(&self) -> String {
        let old_rev = self.old_rev.as_deref().unwrap_or("");
        let new_rev = self.new_rev.as_deref().unwrap_or("");

        if self.old_rev.is_some() && self.new_rev.is_some() {
            format!("{}-{}", old_rev, new_rev)
        } else {
            format!("{}{}", old_rev, new_rev)
        }
    }

    fn from_string(string: &str) -> RevisionRange {
        let mut revision_range = RevisionRange::default();
        let split = string.split("-").take(2).filter(|x| !x.is_empty()).collect::<Vec<_>>();

        if let Some(y) = split.get(1) {
            revision_range.new_rev = Some(y.to_string());

            if let Some(x) = split.first() {
                revision_range.old_rev = Some(x.to_string());
            }
        } else if let Some(x) = split.first() {
            revision_range.new_rev = Some(x.to_string());
        }

        revision_range
    }

    pub(crate) fn append_to_file_name(&self, path: PathBuf) -> Result<PathBuf, Box<dyn Error>> {
        let rev = format!("~{}~", self.format_range());
        let file_stem = path.file_stem().unwrap_or_else(|| OsStr::new(""));
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

    pub(crate) fn extract_from_file_name(path: PathBuf) -> (RevisionRange, PathBuf) {
        let mut revision_range = RevisionRange {
            old_rev: None,
            new_rev: None,
        };
        let file_stem = path.file_stem().unwrap_or_else(|| OsStr::new(""));
        let file_name = file_stem.to_string_lossy();
        let mut split = file_name.split(".").filter(|x| !x.is_empty()).collect::<Vec<_>>();

        if split.len() > 1
            && let Some(revision) = split.pop()
        {
            if revision.starts_with('~') && revision.ends_with('~') {
                let trimmed = revision.trim_matches('~');
                revision_range = RevisionRange::from_string(trimmed);
            } else {
                split.push(revision);
            }
        }

        let mut file_name = split.join(".");

        if let Some(extension) = path.extension() {
            file_name.push('.');
            file_name.push_str(&extension.to_string_lossy());
        }

        (revision_range, path.with_file_name(file_name))
    }
}

#[test]
fn revision_range_to_string_test() {
    let mut a = RevisionRange {
        old_rev: Some("r123".to_owned()),
        new_rev: Some("r456".to_owned()),
    };
    assert_eq!(a.to_string(), "r123-r456");
    a = RevisionRange {
        old_rev: Some("r123".to_owned()),
        new_rev: None,
    };
    assert_eq!(a.to_string(), "r123");
    a = RevisionRange {
        old_rev: Some("r456".to_owned()),
        new_rev: None,
    };
    assert_eq!(a.to_string(), "r456");
    a = RevisionRange {
        old_rev: None,
        new_rev: None,
    };
    assert_eq!(a.to_string(), "");
}

#[test]
fn revision_range_from_string_test() {
    let mut a = RevisionRange::from_string("r123-r456");
    assert_eq!(a.old_rev, Some("r123".to_owned()));
    assert_eq!(a.new_rev, Some("r456".to_owned()));
    a = RevisionRange::from_string("r123");
    assert_eq!(a.old_rev, None);
    assert_eq!(a.new_rev, Some("r123".to_owned()));
    a = RevisionRange::from_string("");
    assert_eq!(a.old_rev, None);
    assert_eq!(a.new_rev, None);
    a = RevisionRange::from_string("-");
    assert_eq!(a.old_rev, None);
    assert_eq!(a.new_rev, None);
}

#[test]
fn revision_range_append_to_file_name_test() {
    let p = PathBuf::from("/test/path/myfile.resx");
    let rev = RevisionRange {
        old_rev: Some("r123".to_owned()),
        new_rev: Some("r456".to_owned()),
    };
    match rev.append_to_file_name(p) {
        Ok(path) => assert_eq!(path, PathBuf::from("/test/path/myfile.~r123-r456~.resx")),
        Err(e) => panic!("append_to_file_name failed: {}", e),
    }
}

#[test]

fn revision_range_extract_from_file_name_test() {
    let (revision, path) = RevisionRange::extract_from_file_name(PathBuf::from("/test/path/myfile.~r123-r456~.resx"));
    assert_eq!(revision.old_rev, Some("r123".to_owned()));
    assert_eq!(revision.new_rev, Some("r456".to_owned()));
    assert_eq!(path, PathBuf::from("/test/path/myfile.resx"));
    let (revision, path) =
        RevisionRange::extract_from_file_name(PathBuf::from("/test/path/myfile.not.a.revision.resx"));
    assert_eq!(revision.old_rev, None);
    assert_eq!(revision.new_rev, None);
    assert_eq!(path, PathBuf::from("/test/path/myfile.not.a.revision.resx"));
}
