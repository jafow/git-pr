// #![feature(with_options)]

use std::env;
use std::fs;

use std::fs::File;
use std::io::{self, prelude::*};
use std::path::Path;
use std::process::Command;

use async_std::task;
use regex::Regex;

// use serde_json::Result;
use surf;

extern crate clap;
use clap::{App, Arg};

const PR_EDITMSG_PATH: &str = ".git/PR_EDITMSG";

fn read_file(head_file: &Path) -> Result<String, io::Error> {
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
    use super::*;
    // override the path to PR_EDITMSG template for testing
    const PR_EDITMSG_PATH: &str = "./tests/PR_EDITMSG";

    #[test]
    fn test_current_branch() {
        let hf = read_file(&Path::new("./tests/HEAD_A")).expect("test file");
        let actual = current_branch(hf);

        assert_eq!(Some(String::from("test-branch")), actual)
    }

    #[test]
    fn test_build_message() -> Result<(), Box<dyn std::error::Error>> {
        // it should build a PullRequestMsg from file
        let mut f = File::create(PR_EDITMSG_PATH)?;
        f.write_all(
            b"test title\n\nthis is a test msg body\n\n// Requesting a pull to master from feat",
        );

        let expected = PullRequestMsg {
            title: String::from("test title"),
            body: String::from("this is a test msg body"),
        };
        assert_eq!(Some(expected), build_pr_msg(Some(PR_EDITMSG_PATH)));
        Ok(())
    }

    #[test]
    fn test_build_request_payload() {
        let test_input = PullRequest {
            target_branch: "master",
            head_branch: "test",
            message: PullRequestMsg {
                title: String::from("test title"),
                body: String::from("this is a test msg body"),
            },
        };

        let expected = serde_json::json!({"title": "test title", "body": "this is a test msg body", "head": "test", "base": "master"});
        assert_eq!(expected, build_request_payload(test_input))
    }
}

fn get_remote(text: &str) -> Result<Vec<&str>, ()> {
    // captures author, repo, and remote url from git config file
    let re = Regex::new(r#"\[remote\s+"(?P<origin>\w+)"\]\n\turl\s=\s(https?://|git@)github.com[:/]?(?P<author>[A-Za-z0-9_]+)/(?P<repo>[A-Za-z0-9_-]+)"#).unwrap();

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

fn launch_editor(pr_file: &str) -> std::io::Result<()> {
    let editor = env::var("GIT_EDITOR").expect("no $GIT_EDITOR set");
    let sub = format!("{} {}", editor, pr_file);
    let cmd = Command::new("sh")
        .args(&["-c", &sub])
        .spawn()
        .and_then(|mut c| c.wait())
        .expect("error opening editor");
    Ok(())
}

fn build_pr_msg(msg_path: Option<&str>) -> Option<PullRequestMsg> {
    let p = match msg_path {
        Some(p) => p,
        None => PR_EDITMSG_PATH,
    };
    let pr_file: String = fs::read_to_string(p).expect("read test file");
    let mut lines = pr_file.lines();
    let mut title = String::new();

    // set the first line as title
    if let Some(_title) = lines.next() {
        title = String::from(_title);
    } else {
        println!("Error getting title");
    }

    let body: String = lines
        .take_while(|line| !line.starts_with("// Requesting a pull to"))
        .collect();

    Some(PullRequestMsg { title, body })
}

fn pr_msg_template(target: &str, current: &str) -> std::io::Result<()> {
    let mut pr_file = File::create(PR_EDITMSG_PATH)?;

    let msg = format!(
        "

// Requesting a pull to {} from {}
// Write a message for this pull request. The first line
// of text is the title and the rest is the description.
// All lines beginning with // will be ignored",
        target, current
    );

    pr_file.write_all(msg.as_bytes()).expect("write pr file");
    Ok(())
}

fn build_request_payload(pr: PullRequest) -> serde_json::Value {
    serde_json::json!(
        {
            "title": pr.message.title,
            "body": pr.message.body,
            "head": pr.head_branch,
            "base": pr.target_branch
        }
    )
}

#[derive(Debug, PartialEq)]
struct PullRequestMsg {
    title: String,
    body: String,
}

#[derive(Debug, PartialEq)]
struct PullRequest<'a> {
    target_branch: &'a str,
    head_branch: &'a str,
    message: PullRequestMsg,
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

    let token = env::var("GITHUB_TOKEN").expect("required GITHUB_TOKEN");

    let git_head = read_file(&Path::new("./.git/HEAD")).expect("git HEAD");
    let config_file = read_file(&Path::new("./.git/config"))?;
    let repo_data = get_remote(&config_file).expect("read git config file");
    let br = current_branch(git_head).unwrap();

    pr_msg_template(&target, &br).expect("build PR message template");

    launch_editor(PR_EDITMSG_PATH).expect("launch editor");

    let msg: PullRequestMsg = build_pr_msg(None).expect("build pr message");

    let pr = PullRequest {
        target_branch: &target,
        head_branch: &br,
        message: msg,
    };

    task::block_on(async {
        let payload = build_request_payload(pr);
        let ff = fetch_api(&repo_data[1], &repo_data[0], &token, payload).await;
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

async fn fetch_api(
    repo: &str,
    uname: &str,
    password: &str,
    body: serde_json::Value,
) -> Result<surf::Response, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let url = format!(
        "https://{}:{}@api.github.com/repos/{}/{}/pulls",
        &uname, &password, &uname, &repo
    );

    // let body = serde_json::json!({"title": format!("\"{}\"", pr_data.message.title), "head": format!("\"{}\"", pr_data.head_branch), "base": pr_data.target_branch, "body": pr_data.message.body});
    let res = surf::post(&url).body_json(&body)?.await?;
    dbg!(&res);
    Ok(res)
}
