
use std::fmt;

#[derive(Clone)]
pub struct Resource {
    pub name: String,
    pub value: String,
}

impl fmt::Debug for Resource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}={}", self.name, self.value)
    }
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}={}", self.name, self.value)
    }
}

impl PartialEq for Resource {
    fn eq(&self, other: &Resource) -> bool {
        (self.name == other.name) && (self.value == other.value)
    }
}

impl Resource {
    pub fn new(name: &str, value: &str) -> Self {
        Resource {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }
}
