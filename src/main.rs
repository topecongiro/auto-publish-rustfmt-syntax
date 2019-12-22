#[macro_use]
extern crate log;

use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write},
    iter::once,
    path::{Path, PathBuf},
};

use cargo_metadata::{Metadata, MetadataCommand};
use structopt::StructOpt;
use walkdir::WalkDir;
use std::fs::create_dir_all;

const CRATES_WHICH_REQUIRES_RUSTC_PRIVATE_FEATURES: &[&str] =
    &["rustc_data_structures", "rustc_session"];

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "rust-src", parse(from_os_str))]
    root: PathBuf,
    #[structopt(short, long, default_value = "rustfmt-syntax", parse(from_os_str))]
    out: PathBuf,
    #[structopt(short, long)]
    force: bool,
    #[structopt(name = "CRATE", required = true)]
    crates: Vec<String>,
}

fn main() -> std::io::Result<()> {
    env_logger::init();

    let opt: Opt = Opt::from_args();

    let mut command = MetadataCommand::new();
    command.current_dir(&opt.root);
    command.no_deps();
    let metadata = command.exec().expect("cargo metadata failed");

    let crates_to_copy = find_crates_to_copy(&metadata, opt.crates.into_iter());
    debug!("Found {} crates", crates_to_copy.len());

    if opt.force {
        fs::remove_dir_all(&opt.out)?;
    }

    let mut cargo_toml_content = "[workspace]\nmembers = [\n".to_owned();
    for krate in crates_to_copy {
        let to = opt.out.clone().join(krate.root_path.file_name().unwrap());
        info!("copying {} from {:?} to {:?}", krate.name, krate.root_path, to);

        copy_dir_all(&krate.root_path, &to)?;

        if CRATES_WHICH_REQUIRES_RUSTC_PRIVATE_FEATURES.contains(&krate.name) {
            add_rustc_private_feature(&krate, &to)?;
        }

        cargo_toml_content.push_str(&format!("  \"{}\",\n", krate.root_path.file_name().unwrap().to_string_lossy()));
    }
    cargo_toml_content.push_str("]\n");

    let cargo_toml_file_path = opt.out.join("Cargo.toml");
    let mut f = fs::File::create(cargo_toml_file_path)?;
    f.write_all(cargo_toml_content.as_bytes())?;

    Ok(())
}

fn find_crates_to_copy(
    metadata: &Metadata,
    crates: impl Iterator<Item = String>,
) -> BTreeSet<LocalCrate<'_>> {
    crates
        .flat_map(|krate| get_local_dependencies_of_crate(metadata, &krate))
        .collect()
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
struct LocalCrate<'a> {
    name: &'a str,
    root_path: &'a Path,
    lib_path: &'a Path,
}

fn get_local_dependencies_of_crate<'a>(
    metadata: &'a Metadata,
    krate: &str,
) -> BTreeSet<LocalCrate<'a>> {
    let package = metadata
        .packages
        .iter()
        .find(|p| p.name == krate || format!("lib{}", p.name) == krate)
        .expect(&format!("Could not find {}", krate));

    let this_crate = LocalCrate {
        name: package.name.as_str(),
        root_path: package
            .manifest_path
            .parent()
            .expect("Manifest path's parent directory does not exist"),
        lib_path: package.targets[0].src_path.as_path(),
    };

    let local_dependencies = package
        .dependencies
        .iter()
        .filter(|dep| dep.source.is_none())
        .flat_map(|dep| get_local_dependencies_of_crate(metadata, &dep.name));

    once(this_crate).chain(local_dependencies).collect()
}

fn copy_dir_all<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    for entry in WalkDir::new(from.as_ref()) {
        let entry = entry?;
        let relative_source_path = entry.path().strip_prefix(from.as_ref()).expect("Invalid path");
        let target_path = to.as_ref().to_path_buf().join(relative_source_path);
        if entry.file_type().is_dir() {
            create_dir_all(target_path)?;
        } else {
            fs::copy(entry.path(), target_path)?;
        }
    }

    Ok(())
}

fn add_rustc_private_feature(source_krate: &LocalCrate<'_>, to: &Path) -> io::Result<()> {
    let to_path = to.join(source_krate.lib_path.file_name().unwrap());
    debug!("Modifying {:?} using {:?}", to_path, source_krate.lib_path);
    let source_content = fs::read_to_string(source_krate.lib_path)?;

    let mut f = fs::File::create(to_path)?;
    let target_content = "#![feature(rustc_private)]\n".to_owned() + &source_content;
    f.write_all(target_content.as_bytes())
}

