use clap::ArgMatches;
use std::path::{Path, PathBuf};

use error;

const CORE_URL: &'static str = "https://github.com/SCAII/SCAII";
const CORE_NAME: &'static str = "SCAII";

const RTS_URL: &'static str = "https://github.com/SCAII/Sky-RTS";
const RTS_NAME: &'static str = "Sky-RTS";

const DEFAULT_BRANCH: &'static str = "master";

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
                scaii_dir.push(".git");
                scaii_dir.push(name);
                scaii_dir
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Get<'a> {
    url: &'a str,
    branch: &'a str,
    path: PathBuf,
    force: bool,
}

impl<'a> Get<'a> {
    pub fn from_subcommand(
        subcommand: &'a ArgMatches<'a>,
        scaii_dir: &Path,
    ) -> error::Result<Self> {
        /* The unwrapping is because clap also *validates* arguments; can't
        be due to user error */
        let resource = subcommand.subcommand();
        let (resource, args) = (resource.0, resource.1.unwrap());

        let save_path = subcommand.value_of("save-path");
        let branch = subcommand.value_of("branch").unwrap_or(DEFAULT_BRANCH);

        let force = subcommand.is_present("force");

        match resource {
            "core" => Ok(Get::new_core(save_path, branch, force, scaii_dir)),
            "rts" => Ok(Get::new_rts(save_path, branch, force, scaii_dir)),
            "backend" => Get::new_backend(
                NameOrPath::try_from_path_or_name(save_path, args.value_of("name")).unwrap(),
                branch,
                force,
                args.value_of("url").unwrap(),
                scaii_dir,
            ),
            _ => usage_and_exit!(subcommand),
        }
    }

    pub fn new_core(
        save_path: Option<&'a str>,
        branch: &'a str,
        force: bool,
        scaii_dir: &Path,
    ) -> Self {
        Get {
            path: NameOrPath::from_path_or_default(save_path, CORE_NAME).to_path_buf(scaii_dir),
            url: CORE_URL,
            branch: branch,
            force,
        }
    }

    pub fn new_rts(
        save_path: Option<&'a str>,
        branch: &'a str,
        force: bool,
        scaii_dir: &Path,
    ) -> Self {
        Get {
            path: NameOrPath::from_path_or_default(save_path, RTS_NAME).to_path_buf(scaii_dir),
            url: RTS_URL,
            branch: branch,
            force,
        }
    }

    pub fn new_backend(
        name_path: NameOrPath<'a>,
        branch: &'a str,
        force: bool,
        url: &'a str,
        scaii_dir: &Path,
    ) -> error::Result<Self> {
        if let NameOrPath::Name(ref name) = name_path {
            if *name == CORE_NAME || *name == RTS_NAME {
                bail!(
                "Use of reserved resource name {} (Note: reserved names are 'SCAII' and 'Sky-RTS')",
                name
                );
            }
        }

        Ok(Get {
            path: name_path.to_path_buf(scaii_dir),
            url: url,
            branch: branch,
            force,
        })
    }

    pub fn get(&self) -> error::Result<()> {
        use std::fs;
        use error::{ErrorKind, ResultExt};

        let install_path = self.path.as_path();

        if install_path.exists() && !self.force {
            bail!(
                "Directory {:?} exists (Hint: rerun this command with '-f' to force overwrite)",
                install_path
            );
        } else if install_path.exists() && self.force {
            fs::remove_dir_all(&install_path)
                .chain_err(|| ErrorKind::CannotCleanError(format!("{:?}", install_path)))?;
        }

        fs::create_dir_all(&install_path)
            .chain_err(|| ErrorKind::CannotCreateError(format!("{:?}", install_path)))?;

        println!(
            "Cloning git repository at '{}' into '{:?}'",
            self.url, install_path
        );

        clone_repo(install_path, &*self.url, &*self.branch)
    }
}

#[cfg(windows)]
fn clone_repo<P: AsRef<Path>>(target: P, url: &str, branch: &str) -> error::Result<()> {
    use std::process::{Command, Stdio};
    use std::fs;
    use walkdir::WalkDir;

    Command::new("git")
        .arg("clone")
        .arg(url)
        .arg("-b")
        .arg(branch)
        .arg(target.as_ref().to_str().unwrap())
        .stdout(Stdio::inherit())
        .output()?;

    // This causes permission bugs if we don't
    // manually set all the files to not be read
    // only

    let wd = WalkDir::new(target);
    for entry in wd {
        let entry = entry?;
        let metadata = entry.metadata()?;

        // Folders are always readonly in windows
        if metadata.is_file() {
            let mut perm = metadata.permissions();
            perm.set_readonly(false);
            fs::set_permissions(entry.path(), perm)?;
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn clone_repo<P: AsRef<Path>>(target: P, url: &str, branch: &str) -> error::Result<()> {
    use git2::build::RepoBuilder;

    RepoBuilder::new()
        .branch(branch)
        .clone(url, target.as_ref())?;

    Ok(())
}
