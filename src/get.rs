use clap::ArgMatches;
use std::path::{Path, PathBuf};

use error;

use util::{CdManager, NameOrPath};
use constants::*;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Get<'a> {
    url: &'a str,
    branch: &'a str,
    path: PathBuf,
    force: bool,
    is_core: bool,
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
            is_core: true,
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
            is_core: false,
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
            is_core: false,
        })
    }

    pub fn get(mut self) -> error::Result<()> {
        use std::fs;
        use fs2;
        use error::{ErrorKind, ResultExt};

        if self.path.exists() && !self.force {
            bail!(
                "Directory {} exists (Hint: rerun this command with '-f' to force overwrite)",
                self.path.display()
            );
        } else if self.path.exists() && self.force {
            fs2::remove_dir_all(&self.path)
                .chain_err(|| ErrorKind::CannotCleanError(format!("{}", self.path.display())))?;
        }

        fs::create_dir_all(&self.path)
            .chain_err(|| ErrorKind::CannotCreateError(format!("{}", self.path.display())))?;

        println!(
            "Cloning git repository at '{}' into '{}'",
            self.url,
            self.path.display()
        );

        clone_repo(&self.path, &*self.url, &*self.branch)?;

        if self.is_core {
            self.get_core_resources()
                .chain_err(|| "Could not fetch core dependencies")
        } else {
            Ok(())
        }
    }

    pub fn get_core_resources(&mut self) -> error::Result<()> {
        use error::ResultExt;

        // Ensures we can't forget to pop our modifications off the path
        let mut path = CdManager::new(&mut self.path);
        path.push("viz/js");

        ensure!(
            path.as_ref().exists(),
            "Cannot find visualization in core, should be at {}",
            path.as_ref().display(),
        );

        let buf = Vec::with_capacity(CLOSURE_LIB_BYTES.max(PROTOBUF_JS_BYTES));
        let mut buf = get_closure_lib(path.layer(), buf)
            .chain_err(|| "Could not fetch Google Closure Library")?;
        buf.clear();
        get_protobuf_js(path.layer(), buf).chain_err(|| "Could not fetch protobuf_js")?;

        Ok(())
    }
}

fn get_closure_lib(mut path: CdManager, buf: Vec<u8>) -> error::Result<Vec<u8>> {
    use util;
    path.push("closure_library");

    let buf = util::curl(CLOSURE_LIB_URL, Some(buf))?;
    util::unzip(&buf, path.layer(), true)?;

    Ok(buf)
}

fn get_protobuf_js(mut path: CdManager, buf: Vec<u8>) -> error::Result<Vec<u8>> {
    use util;
    use std::fs;
    use fs2;

    let buf = util::curl(PROTOBUF_JS_URL, Some(buf))?;
    util::unzip(&buf, path.layer(), false)?;

    let mut curr_dir = path.clone_inner();
    curr_dir.push("protobuf_js");

    path.push("protobuf-3.5.1");

    path.push("js");

    fs::rename(&path, curr_dir)?;

    path.pop()?;
    fs2::remove_dir_all(path)?;

    Ok(buf)
}

#[cfg(windows)]
fn clone_repo<P: AsRef<Path>>(target: P, url: &str, branch: &str) -> error::Result<()> {
    use std::process::{Command, Stdio};

    Command::new("git")
        .arg("clone")
        .arg(url)
        .arg("-b")
        .arg(branch)
        .arg(target.as_ref().to_str().unwrap())
        .stdout(Stdio::inherit())
        .output()?;

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
