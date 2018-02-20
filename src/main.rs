#[macro_use]
extern crate clap;

use clap::App;

fn main() {
    let yaml = load_yaml!("args.yml");
    let app = App::from_yaml(yaml)
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .get_matches();
}
