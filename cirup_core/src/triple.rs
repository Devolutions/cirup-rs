use std::fmt;

#[derive(Clone)]
pub struct Triple {
    pub name: String,
    pub value: String,
    pub base: String,
}

impl fmt::Debug for Triple {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},{},{}", self.name, self.value, self.base)
    }
}

impl fmt::Display for Triple {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},{},{}", self.name, self.value, self.base)
    }
}

impl PartialEq for Triple {
    fn eq(&self, other: &Triple) -> bool {
        (self.name == other.name) && (self.value == other.value)
    }
}

impl Triple {
    pub fn new(name: &str, value: &str, base: &str) -> Self {
        Triple {
            name: name.to_owned(),
            value: value.to_owned(),
            base: base.to_owned(),
        }
    }
}
