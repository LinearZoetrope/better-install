use clap::ArgMatches;
use std::path::{Path, PathBuf};

use util::CdManager;

use error;

use indicatif::ProgressBar;

const CORE_URL: &'static str = "https://github.com/SCAII/SCAII";
const CORE_NAME: &'static str = "SCAII";

const RTS_URL: &'static str = "https://github.com/SCAII/Sky-RTS";
const RTS_NAME: &'static str = "Sky-RTS";

const DEFAULT_BRANCH: &'static str = "master";

const CLOSURE_LIB_URL: &'static str =
    "https://github.com/google/closure-library/archive/v20171112.zip";
const CLOSURE_LIB_BYTES: usize = 7_032_575;

const PROTOBUF_JS_URL: &'static str =
    "https://github.com/google/protobuf/releases/download/v3.5.1/protobuf-js-3.5.1.zip";
const PROTOBUF_JS_BYTES: usize = 5_538_299;

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
        use error::{ErrorKind, ResultExt};
        use indicatif::{ProgressBar, ProgressStyle};
        use console::style;

        if self.path.exists() && !self.force {
            bail!(
                "Directory {:?} exists (Hint: rerun this command with '-f' to force overwrite)",
                self.path
            );
        } else if self.path.exists() && self.force {
            fs::remove_dir_all(&self.path)
                .chain_err(|| ErrorKind::CannotCleanError(format!("{:?}", self.path)))?;
        }

        fs::create_dir_all(&self.path)
            .chain_err(|| ErrorKind::CannotCreateError(format!("{:?}", self.path)))?;

        clone_repo(&self.path, &*self.url, &*self.branch)?;

        if self.is_core {
            self.get_core_resources()
                .chain_err(|| "Could not fetch core dependencies")
        } else {
            Ok(())
        }
    }

    pub fn get_core_resources(&mut self) -> error::Result<()> {
        use error::{ErrorKind, MultiError, ResultExt};
        use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
        use crossbeam;
        use std::sync::mpsc;

        // Ensures we can't forget to pop our modifications off the path
        let mut path = CdManager::new(&mut self.path);
        path.push("viz/js");

        ensure!(
            path.as_ref().exists(),
            "Cannot find visualization in core, should be at {}",
            path.as_ref().display(),
        );

        let mut errors = MultiError { errors: vec![] };

        crossbeam::scope(|scope| {
            let (err_send, err_recv) = mpsc::channel();
            let bars = MultiProgress::new();
            bars.set_move_cursor(true);

            let closure_bar = bars.add(ProgressBar::new_spinner());
            closure_bar.set_style(
                ProgressStyle::default_spinner()
                    .template("[{elapsed_precise}] {spinner} {msg}")
                    .tick_chars(r"/\"),
            );
            let closure_err = err_send.clone();
            let mut closure_path = path.clone_inner();

            let protobuf_bar = bars.add(ProgressBar::new_spinner());
            protobuf_bar.set_style(
                ProgressStyle::default_spinner()
                    .template("[{elapsed_precise}] {spinner} {msg}")
                    .tick_chars(r"/\"),
            );
            let protobuf_err = err_send;
            let mut protobuf_path = path.clone_inner();

            let mut joins = Vec::with_capacity(2);
            joins.push(scope.spawn(move || {
                let closure_path = CdManager::new(&mut closure_path);

                closure_err
                    .send(get_closure_lib(closure_path, closure_bar))
                    .expect("Could not send closure error");
            }));

            joins.push(scope.spawn(move || {
                let protobuf_path = CdManager::new(&mut protobuf_path);

                protobuf_err
                    .send(get_protobuf_js(protobuf_path, protobuf_bar))
                    .expect("Could not send closure error");
            }));

            for res in err_recv.into_iter().take(2) {
                if let Err(err) = res {
                    errors.errors.push(err);
                }
            }

            if let Err(err) = bars.join_and_clear()
                .chain_err(|| "Error joining progress bars")
            {
                errors.errors.push(err);
            }
        });

        if errors.errors.len() > 0 {
            bail!(ErrorKind::MultiError(errors))
        } else {
            Ok(())
        }
    }
}

fn get_closure_lib(mut path: CdManager, bar: ProgressBar) -> error::Result<()> {
    use indicatif::ProgressStyle;
    use util;
    path.push("closure_library");

    bar.enable_steady_tick(100);
    bar.set_message("Downloading Google Closure Library");
    let buf = Vec::with_capacity(CLOSURE_LIB_BYTES);
    let buf = util::curl(CLOSURE_LIB_URL, Some(buf))?;

    bar.disable_steady_tick();
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} ({percent}) {msg}"),
    );
    bar.set_message("Unzipping Google Closure Library");
    util::unzip(&buf, path.layer(), true, &bar)?;
    #[cfg(windows)]
    {
        util::make_deletable(&path)?;
    }

    bar.finish_with_message("Done downloading Google Closure Library");

    Ok(())
}

fn get_protobuf_js(mut path: CdManager, bar: ProgressBar) -> error::Result<()> {
    use indicatif::ProgressStyle;
    use util;
    use std::fs;

    bar.enable_steady_tick(100);
    bar.set_message("Downloading Protobuf Library");
    let buf = Vec::with_capacity(PROTOBUF_JS_BYTES);
    let buf = util::curl(PROTOBUF_JS_URL, Some(buf))?;

    bar.disable_steady_tick();
    bar.set_message("Unzipping Protobuf Library");
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} ({percent}) {msg}"),
    );
    util::unzip(&buf, path.layer(), false, &bar)?;

    let mut curr_dir = path.clone_inner();
    curr_dir.push("protobuf_js");

    path.push("protobuf-3.5.1");

    #[cfg(windows)]
    {
        util::make_deletable(&path)?;
    }

    path.push("js");

    bar.set_style(ProgressStyle::default_spinner());
    bar.set_message("Placing Protobuf files in the right place...");
    bar.enable_steady_tick(10);

    fs::rename(&path, curr_dir)?;

    path.pop()?;
    fs::remove_dir_all(path)?;

    bar.finish_with_message("Done downloading Protobuf Library");

    Ok(())
}

#[cfg(windows)]
fn clone_repo<P: AsRef<Path>>(target: P, url: &str, branch: &str) -> error::Result<()> {
    use std::process::{Command, Stdio};
    use util;
    use indicatif::ProgressStyle;
    use std::thread;
    use std::time::Duration;

    let bar = ProgressBar::new(0);
    bar.set_message(&format!(
        "Cloning git repository at '{}' into '{}'",
        url,
        target.as_ref().display()
    ));
    bar.set_prefix("[1/?]:");
    bar.set_style(
        ProgressStyle::default_bar()
            .template("{prefix.cyan} [{elapsed.green}] {spinner:2} {msg}")
            .tick_chars(r"/\"),
    );

    let mut child = Command::new("git")
        .arg("clone")
        .arg(url)
        .arg("-b")
        .arg(branch)
        .arg(target.as_ref().to_str().unwrap())
        .spawn()?;

    loop {
        match child.try_wait() {
            Ok(Some(code)) => {
                if code.success() {
                    bar.finish();
                    break;
                } else {
                    bar.finish();
                    bail!("Could not execute git")
                }
            }
            Ok(None) => {
                bar.tick();
                thread::sleep(Duration::from_millis(100))
            }
            Err(err) => {
                bar.finish();
                bail!(err)
            }
        }
    }

    // This causes permission bugs if we don't
    // manually set all the files to not be read
    // only

    util::make_deletable(target)
}

#[cfg(not(windows))]
fn clone_repo<P: AsRef<Path>>(target: P, url: &str, branch: &str) -> error::Result<()> {
    use git2::build::RepoBuilder;

    RepoBuilder::new()
        .branch(branch)
        .clone(url, target.as_ref())?;

    Ok(())
}
