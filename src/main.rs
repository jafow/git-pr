use std::env;
use std::path::Path;

use clap::{App, Arg};
use async_std::task;

const PR_EDITMSG_PATH: &str = ".git/PR_EDITMSG";

mod gitpr;

fn main() -> std::io::Result<()> {
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
                        .required(false)
                        )
                    .get_matches();

    let remote = match matches.value_of("remote") {
        Some(r) => r,
        None => "origin"
    };
    let target = match matches.value_of("target") {
        Some(t) => t,
        None => "master",
    };
    let token = env::var("GITHUB_TOKEN").expect("required GITHUB_TOKEN");
    let git_head = gitpr::read_file(&Path::new("./.git/HEAD")).expect("git HEAD");
    let config_file = gitpr::read_file(&Path::new("./.git/config"))?;
    // todo: destructure this into author and repo only
    let repo_data: gitpr::RepoData = gitpr::repo_config(&config_file, remote).expect("read git config file");
    dbg!(&repo_data.repo_name);
    let branch = gitpr::current_branch(git_head).unwrap();

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
        let ff = gitpr::fetch_api(&repo_data.repo_name, &repo_data.author, &token, payload).await;
        dbg!(&ff);
        match ff {
            Ok(r) => {
                println!("got a response {:?}", r);
                r
            }
            Err(e) => panic!("Error {}", e),
        }
    });

    Ok(())
}
