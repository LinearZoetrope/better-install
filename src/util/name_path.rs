use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum NameOrPath<'a> {
    Name(&'a str),
    SavePath(&'a Path),
}

impl<'a> NameOrPath<'a> {
    pub fn from_path_or_default(path: Option<&'a str>, name: &'a str) -> Self {
        match path {
            Some(path) => NameOrPath::SavePath(Path::new(path)),
            None => NameOrPath::Name(name),
        }
    }

    pub fn try_from_path_or_name(path: Option<&'a str>, name: Option<&'a str>) -> Result<Self, ()> {
        match (path, name) {
            (Some(path), None) => Ok(NameOrPath::SavePath(Path::new(path))),
            (None, Some(name)) => Ok(NameOrPath::Name(name)),
            _ => Err(()),
        }
    }

    pub fn to_path_buf(self, scaii_dir: &Path) -> PathBuf {
        match self {
            NameOrPath::SavePath(path) => path.to_path_buf(),
            NameOrPath::Name(name) => {
                let mut scaii_dir = scaii_dir.to_path_buf();
                scaii_dir.push("git");
                scaii_dir.push(name);
                scaii_dir
            }
        }
    }
}
