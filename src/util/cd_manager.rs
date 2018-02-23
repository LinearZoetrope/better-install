use std::path::{Path, PathBuf};
use error;

/// A "Change Directory" manager or `CdManager` prevents you from forgetting to pop
/// directories at the end of a block.
///
/// It takes a reference to a `PathBuf` and, upon going out of scope, will manually `pop`
/// all elements of the `PathBuf` off that were added during its life.
///
/// The only supported operations are `push` or `pop`, more complex operations such as
/// cannot easily be managed.
///
/// Note that the `CdManager` uses a path's `Components` to determine how many times
/// to call `pop`, so this may cause some inconsistency if your path includes `.`.
///
/// A `CdManager` implements `AsRef<Path>` so it may be used anywhere a `Path` is needed.
#[derive(Debug)]
pub struct CdManager<'a> {
    path: &'a mut PathBuf,
    added_depth: usize,
}

impl<'a> CdManager<'a> {
    /// Starts a new context from the given `PathBuf`
    pub fn new(path: &'a mut PathBuf) -> Self {
        CdManager {
            path,
            added_depth: 0,
        }
    }

    /// Pushes a `Path` onto the `PathBuf`, to be undone when the
    /// `CdManager` goes out of scope.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// let mut path = PathBuf::from("a/path/to/something".to_string());
    /// let mut p2 = path.clone();
    ///
    /// {
    ///     let manager = CdManager::new(&mut p2);
    ///
    ///     path.push("foo/bar");
    ///     manager.push("foo/bar");
    ///
    ///     assert_equal!(path, manager);
    /// } // Automatically pop "foo" from the manager
    ///
    /// path.pop();
    /// path.pop();
    /// assert_eq!(path, p2);
    /// ```
    pub fn push<P: AsRef<Path>>(&mut self, path: P) {
        self.added_depth += path.as_ref().components().count();
        self.path.push(path);
    }

    /// Pops a single link from the underlying `PathBuf`.
    /// This will return an error if this is identical to the
    /// `PathBuf` the `CdManager` was constructured with (that is,
    /// the number of `pop` calls matches the number of pushed `Path` components).
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// let mut path = PathBuf::from("a/path".to_string());
    /// let mut p2 = path.clone();
    /// {
    ///     let mut cd = CdManager::new(&mut p2);
    ///     cd.push("foo");
    ///
    ///     cd.pop().unwrap();
    ///     assert_eq!(cd, path);
    ///
    ///     assert!(cd.pop().is_err());
    /// }
    ///
    /// assert_eq!(path, p2);
    /// ```
    pub fn pop(&mut self) -> error::Result<()> {
        ensure!(
            self.added_depth > 0,
            "Cannot pop off CdManager, going below original directory"
        );

        self.added_depth -= 1;
        self.path.pop();

        Ok(())
    }

    /// Creates a new "layer" of the manager, for scoping purposes
    ///
    /// That is, if you call `CdManager.layer()` in a loop body or function call,
    /// it ensures that any behavior done to the returned `CdManager` will be
    /// undone for you.
    ///
    /// You can think of this as a cheaper, scoped `Clone`.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// let mut path = PathBuf::from("a/path".to_string());
    /// let mut p2 = path.clone();
    ///
    /// let cd = CdManager::new(&mut p2);
    ///
    /// for _ in 0..100 {
    ///     assert_eq!(cd, path);
    ///     let mut cd = cd.layer();
    ///     cd.push("bar");
    ///
    ///     path.push("bar");
    ///     assert_eq!(cd, path);
    ///     path.pop()
    /// }
    /// ```
    pub fn layer(&mut self) -> CdManager {
        CdManager::new(&mut self.path)
    }

    ///
    pub fn clone_inner(&self) -> PathBuf {
        self.path.clone()
    }
}

impl<'a, P: AsRef<Path>> PartialEq<P> for CdManager<'a> {
    fn eq(&self, other: &P) -> bool {
        self.path.as_path() == other.as_ref()
    }
}

impl<'a> Eq for CdManager<'a> {}

impl<'a> PartialEq<CdManager<'a>> for PathBuf {
    fn eq(&self, other: &CdManager) -> bool {
        self == other.path
    }
}

impl<'a> Drop for CdManager<'a> {
    fn drop(&mut self) {
        for _ in 0..self.added_depth {
            self.path.pop();
        }
    }
}

impl<'a> AsRef<Path> for CdManager<'a> {
    fn as_ref(&self) -> &Path {
        self.path
    }
}

#[cfg(test)]
mod test {
    use super::CdManager;
    use std::path::PathBuf;

    #[test]
    fn cd_manager_push() {
        let mut path = PathBuf::from("a/path/to/something".to_string());
        let mut p2 = path.clone();

        {
            let mut cd_manager = CdManager::new(&mut p2);

            cd_manager.push("abc/def");
            path.push("abc/def");

            assert_eq!(cd_manager.added_depth, 2);
            assert_eq!(path, cd_manager);

            path.pop();
            path.pop();
        }

        assert_eq!(p2, path);
    }

    #[test]
    fn cd_manager_pop() {
        let mut path = PathBuf::from("a/path/to/something".to_string());
        let mut p2 = path.clone();

        {
            let mut cd_manager = CdManager::new(&mut p2);

            cd_manager.push("abc/def");
            path.push("abc/def");

            cd_manager.pop().unwrap();
            path.pop();

            assert_eq!(path, cd_manager);
            assert_eq!(cd_manager.added_depth, 1);

            path.pop();
        }

        assert_eq!(p2, path);
    }

    #[test]
    fn cd_manager_error() {
        let mut path = PathBuf::from("a/path/to/something".to_string());
        let mut cd_manager = CdManager::new(&mut path);

        assert!(cd_manager.pop().is_err());
    }
}
