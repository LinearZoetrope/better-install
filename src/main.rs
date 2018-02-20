#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;

#[cfg(unix)]
extern crate git2;

use clap::App;

// Important! Macros can only be used after they're defined
// keep this at the top of the imports
#[macro_use]
pub(crate) mod macros;

pub(crate) mod get;

pub(crate) mod error;

use error::Result;

quick_main!{ || -> Result<i32> {
    use get::Get;
    use std::env;
    use error::{ResultExt,ErrorKind, CLEAN_EXIT};

    let yaml = load_yaml!("args.yml");
    let app = App::from_yaml(yaml)
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .get_matches();

    let sub_command = app.subcommand();
    let sub_command = (sub_command.0, sub_command.1.unwrap());

    let mut scaii_home = env::home_dir().expect("No home directory present on this user, aborting");
    scaii_home.push(".scaii");

    match sub_command {
        ("get", sc) => {
            let cmd = Get::from_subcommand(&sc, &scaii_home).chain_err(|| ErrorKind::GetFailure)?;
            cmd.get().chain_err(|| ErrorKind::GetFailure)?;
        }
        ("install", _sc) => unimplemented!(),
        ("clean", _sc) => unimplemented!(),
        _ => unreachable!(),
    };

    Ok(CLEAN_EXIT)
}}
