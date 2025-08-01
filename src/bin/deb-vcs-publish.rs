use breezyshim::controldir::open;
use breezyshim::error::Error;
use breezyshim::prelude::*;
use breezyshim::workingtree;
use clap::Parser;
use debian_analyzer::publish::{create_vcs_url, update_official_vcs};
use debian_changelog::get_maintainer;

use debian_analyzer::get_committer;

use std::io::Write as _;

#[derive(clap::Args, Clone, Debug)]
#[group()]
struct OutputArgs {}

#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    /// Enable debug output
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Print user identity that would be used when committing
    #[arg(long, default_value_t = false)]
    identity: bool,

    /// directory to run in
    #[arg(short, long, default_value = std::env::current_dir().unwrap().into_os_string(), value_name = "DIR")]
    directory: std::path::PathBuf,

    /// Do not create the repository
    #[arg(default_value_t = false)]
    no_create: bool,

    #[arg(default_value_t = false)]
    force: bool,

    /// Push branch
    #[arg(default_value_t = false)]
    push: bool,

    url: Option<url::Url>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    env_logger::builder()
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .filter(
            None,
            if args.debug {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            },
        )
        .init();

    breezyshim::init();

    let (wt, subpath) = match workingtree::open_containing(&args.directory) {
        Ok((wt, subpath)) => (wt, subpath.display().to_string()),
        Err(Error::NotBranchError(_msg, _)) => {
            log::error!("No version control directory found (e.g. a .git directory).");
            std::process::exit(1);
        }
        Err(Error::DependencyNotPresent(name, _reason)) => {
            log::error!(
                "Unable to open tree at {}: missing package {}",
                args.directory.display(),
                name
            );
            std::process::exit(1);
        }
        Err(e) => {
            log::error!("Unable to open tree at {}: {}", args.directory.display(), e);
            std::process::exit(1);
        }
    };
    if args.identity {
        println!("Committer identity: {}", get_committer(&wt));
        let (maintainer, email) = get_maintainer().unwrap_or(("".to_string(), "".to_string()));
        println!("Changelog identity: {} <{}>", maintainer, email);
        std::process::exit(0);
    }

    let parsed_vcs = match update_official_vcs(
        &wt,
        std::path::Path::new(subpath.as_str()),
        args.url.as_ref(),
        None,
        None,
        Some(args.force),
    ) {
        Ok(o) => o,
        Err(e) => {
            log::error!("Unable to update official VCS: {}", e);
            std::process::exit(1);
        }
    };

    let repo_url: url::Url = parsed_vcs.repo_url.parse().unwrap();

    if !args.no_create {
        match create_vcs_url(&repo_url, parsed_vcs.branch.as_deref()) {
            Ok(()) => {}
            Err(Error::UnsupportedForge(_)) => {
                log::error!("Unable to find a way to create {}", repo_url);
            }
            Err(Error::ForgeProjectExists(..)) | Err(Error::AlreadyControlDir(..)) => {
                log::error!("Unable to create {}: already exists", repo_url);
                std::process::exit(1);
            }
            Err(Error::ForgeLoginRequired) => {
                log::error!("Unable to create {}: login required", repo_url);
                std::process::exit(1);
            }
            Err(e) => {
                log::error!("Unable to create {}: {}", repo_url, e);
                std::process::exit(1);
            }
        }
    }

    let controldir = open(&repo_url, None).unwrap();
    let branch = match controldir.open_branch(parsed_vcs.branch.as_deref()) {
        Ok(branch) => branch,
        Err(Error::NotBranchError(_, _)) => controldir
            .create_branch(parsed_vcs.branch.as_deref())
            .unwrap(),
        Err(e) => {
            log::error!("Unable to open or create branch: {}", e);
            std::process::exit(1);
        }
    };

    wt.branch()
        .push(branch.as_ref(), false, None, None)
        .unwrap();
    Ok(())
}
