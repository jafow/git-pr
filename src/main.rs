use std::env;
use std::path::Path;
use std::process;

use async_std::task;
use clap::{App, Arg};

const PR_EDITMSG_PATH: &str = ".git/PR_EDITMSG";

mod gitpr;
use gitpr::*;

macro_rules! werr {
    ($($arg:tt)*) => ({
        use std::io::Write;
        (writeln!(&mut ::std::io::stderr(), $($arg)*)).unwrap();
    });
}

macro_rules! wout {
    ($($arg:tt)*) => ({
        use std::io::Write;
        (writeln!(&mut ::std::io::stdout(), $($arg)*)).unwrap();
    });
}
pub type PullRequestResult<T> = Result<T, PrError>;

fn main() -> PullRequestResult<()> {
    let matches = App::new("git-pr")
                    .version("0.1.0")
                    .author("Jared Fowler <jaredafowler@gmail.com>")
                    .about("Open github pull requests")
                    .usage("git pr origin master\n    git pr upstream feat/add-feature -m \"This is the title of my PR\"")
                    .arg(
                        Arg::with_name("remote")
                        .help("the name of the remote; e.g origin")
                        .index(1)
                        .required(false),
                    )
                    .arg(
                        Arg::with_name("target")
                        .help("[optional] The pull request's target branch; e.g. master. Defaults to master")
                        .index(2)
                        .requires("remote")
                        )
                    .arg(
                        Arg::with_name("message")
                        .help("Use the message as Pull Request message")
                        .short("m")
                        .long("message")
                        .takes_value(true)
                        .required(false)
                        )
                    .get_matches();

    let remote = match matches.value_of("remote") {
        Some(r) => r,
        None => "origin",
    };
    let target = match matches.value_of("target") {
        Some(t) => t,
        None => "master",
    };
    let token = env::var("GITHUB_TOKEN").expect("no GITHUB_TOKEN found in environment");

    let config_file = gitpr::read_file(&Path::new("./.git/config"))?;
    let repo_data: RepoData =
        gitpr::repo_config(&config_file, remote).expect("read git config file");

    let branch = gitpr::branch(&Path::new("./.git/HEAD"))?;
    gitpr::pr_msg_template(&target, &branch).expect("build PR message template");

    gitpr::launch_editor(PR_EDITMSG_PATH).expect("launch editor");

    let msg: gitpr::PullRequestMsg = gitpr::build_pr_msg(None).expect("build pr message");

    let pr = gitpr::PullRequest {
        target_branch: &target,
        head_branch: &branch,
        message: msg,
    };

    task::block_on(async {
        let payload = gitpr::build_request_payload(pr);
        match gitpr::fetch_api(repo_data, &token, payload).await {
            Ok(r) => {
                wout!("Pull request opened at {:?}", r.html_url);
            }
            Err(e) => {
                match e {
                    PrError::Io(err) => werr!("Error: {:?}", err),
                    PrError::Api(err) => werr!("Error calling API: {:?}", err),
                    PrError::Other(err) => werr!("Base exception: {:?}", err),
                    _ => werr!("Unknown exception: {:?}", e)
                }
                process::exit(1);
            }
        }
    });

    Ok(())
}
