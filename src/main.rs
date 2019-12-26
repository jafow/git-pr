// #![feature(with_options)]

use std::env;
use std::fs;

use std::fs::{File, OpenOptions};
use std::io::{self, prelude::*};
use std::path::Path;
use std::process::Command;

use async_std::task;
use regex::Regex;

// use serde_json::Result;

extern crate clap;
use clap::{App, Arg};

const PR_EDITMSG_PATH: &str = ".git/PR_EDITMSG";

fn head_file(head_file: &Path) -> Result<String, io::Error> {
    fs::read_to_string(head_file)
}

/// read over the .git/HEAD file to get current branch
fn current_branch(head_file_contents: String) -> Option<String> {
    match head_file_contents.lines().next() {
        Some(line) => line
            .split('/')
            .map(String::from)
            .collect::<Vec<String>>()
            .pop(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_current_branch() {
        let hf = head_file(&Path::new("./tests/HEAD_A")).expect("test file");
        let actual = current_branch(hf);

        assert_eq!(Some(String::from("test-branch")), actual)
    }
}

fn get_remote(text: &str) -> Result<Vec<&str>, ()> {
    // captures author, repo, and remote url from git config file
    let re = Regex::new(r#"\[remote\s+"(?P<origin>\w+)"\]\n\turl\s=\s(https?://|git@)github.com[:/]?(?P<author>[A-Za-z0-9_]+)/(?P<repo>[A-Za-z0-9_])"#).unwrap();

    let mut v: Vec<&str> = Vec::new();

    for caps in re.captures_iter(text) {
        match &caps.name("author") {
            Some(m) => v.push(m.as_str()),
            None => (),
        }

        match &caps.name("repo") {
            Some(m) => v.push(m.as_str()),
            None => (),
        }

        match &caps.name("origin") {
            Some(m) => v.push(m.as_str()),
            None => (),
        }
    }

    Ok(v)
}

fn launch_editor() -> std::io::Result<()> {
    let editor = env::var("GIT_EDITOR").expect("no $GIT_EDITOR set");
    let sub = format!("{} ", editor);
    let cmd = Command::new("sh")
                .args(&["-c", &sub])
                .spawn()
                .and_then(|mut c| c.wait())
                .expect("error opening editor");

    // let ecode = cmd.wait().expect("open editor failed");

    // dbg!(output);
    dbg!(cmd);
    Ok(())
}


fn build_message(target: &str, current: &str) -> std::io::Result<()> {
    let mut pr_file = File::create(PR_EDITMSG_PATH)?;

    let msg = format!("

// Requesting a pull to {} from {}
// Write a message for this pull request. The first line
// of text is the title and the rest is the description.
// All lines beginning with // will be ignored", target, current);

    pr_file.write_all(msg.as_bytes()).expect("write pr file");
    Ok(())
}

fn build_request(target: &str, current: &str, token: String) {

}

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
                        .required(true),
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

    let remote = matches.value_of("remote").unwrap();
    let target = match matches.value_of("target") {
        Some(t) => t,
        None => "master",
    };

    let git_head = head_file(&Path::new("./.git/HEAD")).expect("git HEAD");
    let br = current_branch(git_head).unwrap();
    let token = env::var("GITHUB_TOKEN").expect("required GITHUB_TOKEN");

    build_message(target, &br)?;
    launch_editor().expect("launch editor");
    // build_request(target, &br, token);
    task::block_on(async {
        let ff = fetch_api("jafow", &token, "git-pr").await;
        match ff {
            Ok(r) => r,
            Err(e) => panic!("Error {}", e)
        }
    });


    Ok(())
}

async fn fetch_api(uname: &str, password: &str, repo: &str) -> Result<surf::Response, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let url = format!("https://{}:{}@api.github.com/repos/{}/{}/pulls", &uname, &password, &uname, &repo);

    let body = serde_json::json!({"title": "foo 3", "head": "feat", "base": "master", "body": "from serde_json"});
    let res = surf::post(&url).body_json(&body)?.await?;
    Ok(res)
}
