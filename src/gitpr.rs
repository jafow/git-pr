use std::env;
use std::fmt;
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

#[derive(Debug, Clone, PartialEq)]
pub struct RepoData<'a> {
    author: &'a str,
    pub repo_name: &'a str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepoError;

impl fmt::Display for RepoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error reading repo configuration")
    }
}

pub fn read_file(head_file: &Path) -> Result<String, io::Error> {
    fs::read_to_string(head_file)
}

/// split the .git/HEAD file on '/' to get current branch
pub fn current_branch(head_file_contents: String) -> Option<String> {
    match head_file_contents.lines().next() {
        Some(line) => line
            .split('/')
            .map(String::from)
            .collect::<Vec<String>>()
            .pop(),
        _ => panic!("Could not find current branch from git config"),
    }
}

#[test]
fn test_current_branch() {
    let hf = read_file(&Path::new("./tests/HEAD_A")).expect("test file");
    let actual = current_branch(hf);

    assert_eq!(Some(String::from("test-branch")), actual);
}

#[test]
#[should_panic]
fn test_current_branch_errors() {
    // it should panic if file doesn't exist or is malformed
    let hf = read_file(&Path::new("./tests/HEAD_FILENOTFOUND")).expect("test file");
    current_branch(hf);

    let hf = read_file(&Path::new("./tests/HEAD_BROKEN")).expect("test file");
    current_branch(hf);
}

pub fn build_pr_msg(msg_path: Option<&str>) -> Option<PullRequestMsg> {
    let p = match msg_path {
        Some(p) => p,
        None => PR_EDITMSG_PATH,
    };
    let mut title = String::new();

    let pr_file: String = fs::read_to_string(p).expect("read test file");
    let mut lines = pr_file.lines();

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

pub fn repo_config<'a>(text: &'a str, remote_match: &'a str) -> Result<RepoData<'a>, RepoError> {
    // captures author, repo, and remote url from git config file
    let re = Regex::new(r#"\[remote\s+"(?P<origin>\w+)"\]\n\turl\s=\s(https?://|git@)github.com[:/]?(?P<author>[A-Za-z0-9_-]+)/(?P<repo>[A-Za-z0-9_-]+)"#).unwrap();

    // get only lines that match on the remote provided so that we
    // avoid confusion with forked remotes of the same name
    let match_lines = match re
        .captures(text)
        .filter(|c| c.name("origin").unwrap().as_str() == remote_match)
    {
        Some(m) => m,
        None => return Err(RepoError),
    };

    Ok(RepoData {
        author: match match_lines.get(3) {
            Some(s) => s.as_str(),
            None => return Err(RepoError),
        },
        repo_name: match match_lines.get(4) {
            Some(s) => s.as_str(),
            None => return Err(RepoError),
        },
    })
}

#[test]
fn test_repo_config() {
    // it should pull RepoData from config
    let cfg_file = r#"
[core]
	bare = false
	repositoryformatversion = 0
	filemode = true
	logallrefupdates = true
[remote "origin"]
	url = git@github.com:jafow/git-pr.git
	fetch = +refs/heads/*:refs/remotes/origin/*
[branch "master"]
	remote = origin
	merge = refs/heads/master
"#;
    assert_eq!(
        Ok(RepoData {
            author: "jafow",
            repo_name: "git-pr"
        }),
        repo_config(cfg_file, "origin")
    );

    let cfg_file = r#"
[core]
	bare = false
	repositoryformatversion = 0
	filemode = true
	logallrefupdates = true
[remote "origin"]
	url = unrecognizable-url/jafow/git-pr.git
	fetch = +refs/heads/*:refs/remotes/origin/*
[branch "master"]
	remote = origin
	merge = refs/heads/master
"#;
    assert_eq!(
        Err(RepoError),
        repo_config(cfg_file, "upstream"),
        "Error on non existent remote"
    );
    assert_eq!(
        Err(RepoError),
        repo_config(cfg_file, "origin"),
        "Error on malformed url"
    );
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
    let _cmd = Command::new("sh")
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

    assert_eq!(
        serde_json::json!({"title": "test title", "body": "this is a test msg body", "head": "test", "base": "master"}),
        build_request_payload(test_input)
    )
}

pub async fn fetch_api<'a>(
    repo_data: RepoData<'a>,
    token: &str,
    body: serde_json::Value,
) -> Result<surf::Response, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let url = format!(
        "https://{}:{}@api.github.com/repos/{}/{}/pulls",
        &repo_data.author, &token, &repo_data.author, &repo_data.repo_name
    );

    dbg!(&url);

    let res = surf::post(&url).body_json(&body)?.await?;
    dbg!(&res);
    Ok(res)
}
