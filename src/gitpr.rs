use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use regex::Regex;
use surf;

const PR_EDITMSG_PATH: &str = ".git/PR_EDITMSG";

#[derive(Debug, PartialEq)]
pub struct PullRequestMsg {
    pub title: String,
    pub body: String,
}

#[derive(Debug, PartialEq)]
pub struct PullRequest<'a> {
    pub target_branch: &'a str,
    pub head_branch: &'a str,
    pub message: PullRequestMsg,
}


pub struct RepoData {
    pub author: String,
    pub repo_name: String,
    pub remote: String
}

pub fn read_file(head_file: &Path) -> Result<String, io::Error> {
    fs::read_to_string(head_file)
}

/// read over the .git/HEAD file to get current branch
pub fn current_branch(head_file_contents: String) -> Option<String> {
    match head_file_contents.lines().next() {
        Some(line) => line
            .split('/')
            .map(String::from)
            .collect::<Vec<String>>()
            .pop(),
        _ => None,
    }
}

#[test]
fn test_current_branch() {
    let hf = read_file(&Path::new("./tests/HEAD_A")).expect("test file");
    let actual = current_branch(hf);

    assert_eq!(Some(String::from("test-branch")), actual)
}

pub fn build_pr_msg(msg_path: Option<&str>) -> Option<PullRequestMsg> {
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

    dbg!(&body);

    Some(PullRequestMsg { title, body })
}

#[test]
fn test_build_message() -> Result<(), Box<dyn std::error::Error>> {
    // it should build a PullRequestMsg from file
    const PR_EDITMSG_PATH: &str = "./tests/PR_EDITMSG";
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

pub fn repo_config(text: &str, remote_match: &str) -> Result<RepoData, Box<dyn std::error::Error>> {
    // captures author, repo, and remote url from git config file
    let re = Regex::new(r#"\[remote\s+"(?P<origin>\w+)"\]\n\turl\s=\s(https?://|git@)github.com[:/]?(?P<author>[A-Za-z0-9_]+)/(?P<repo>[A-Za-z0-9_-]+)"#).unwrap();

    let mut author = String::new();
    let mut repo_name = String::new();
    let mut remote = String::new();

    for caps in re.captures_iter(text) {
        match &caps.name("author") {
            Some(m) => author.push_str(m.as_str()),
            None => (),
        }

        match &caps.name("repo") {
            Some(m) => repo_name.push_str(m.as_str()),
            None => (),
        }

        match &caps.name("origin") {
            Some(m) => {
                let ms = m.as_str();
                if ms == remote_match {
                    remote.push_str(ms);
                }
            },
            None => (),
        }
    }

    Ok(RepoData { author, repo_name, remote })
}

#[test]
fn test_repo_config() {
    // it should get the author, repo, and remote from config

}

pub fn pr_msg_template(target: &str, current: &str) -> std::io::Result<()> {
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

pub fn launch_editor(pr_file: &str) -> std::io::Result<()> {
    let editor = env::var("GIT_EDITOR").expect("no $GIT_EDITOR set");
    let sub = format!("{} {}", editor, pr_file);
    let cmd = Command::new("sh")
        .args(&["-c", &sub])
        .spawn()
        .and_then(|mut c| c.wait())
        .expect("error opening editor");
    Ok(())
}

pub fn build_request_payload(pr: PullRequest) -> serde_json::Value {
    serde_json::json!(
        {
            "title": pr.message.title,
            "body": pr.message.body,
            "head": pr.head_branch,
            "base": pr.target_branch
        }
    )
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

pub async fn fetch_api(
    repo: &str,
    uname: &str,
    password: &str,
    body: serde_json::Value,
) -> Result<surf::Response, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let url = format!(
        "https://{}:{}@api.github.com/repos/{}/{}/pulls",
        &uname, &password, &uname, &repo
    );

    dbg!(&url);

    let res = surf::post(&url).body_json(&body)?.await?;
    dbg!(&res);
    Ok(res)
}
