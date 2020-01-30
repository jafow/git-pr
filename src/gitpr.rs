use std::env;
use std::error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use regex::Regex;
use serde::Deserialize;
use surf::{self, http};

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

#[derive(Debug, PartialEq)]
pub enum PrError {
    Api(String),
    Repo(String),
    De(String),
    Io(String),
    Other(String),
}

impl fmt::Display for PrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PrError::Api(ref e) => e.fmt(f),
            PrError::Repo(ref e) => e.fmt(f),
            PrError::Io(ref e) => e.fmt(f),
            PrError::De(ref e) => e.fmt(f),
            PrError::Other(ref s) => f.write_str(&**s),
        }
    }
}

impl error::Error for PrError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl From<io::Error> for PrError {
    fn from(_x: io::Error) -> PrError {
        PrError::Io(_x.to_string())
    }
}

impl From<serde_json::error::Error> for PrError {
    fn from(_x: serde_json::error::Error) -> PrError {
        PrError::De(_x.to_string())
    }
}

impl From<std::boxed::Box<dyn error::Error + std::marker::Send + std::marker::Sync>> for PrError {
    fn from(x: Box<dyn error::Error + std::marker::Send + std::marker::Sync>) -> PrError {
        PrError::Other(x.to_string())
    }
}


pub fn read_file(head_file: &Path) -> Result<String, io::Error> {
    fs::read_to_string(head_file)
}

pub fn branch(head_file: &Path) -> Result<String, PrError> {
    match read_file(head_file) {
        Ok(f) => current_branch(f),
        Err(e) => Err(PrError::Io(format!("Cannot read .git HEAD file; {}", e.to_string()))),
    }
}

/// split the .git/HEAD file on '/' to get current branch
fn current_branch(head_file_contents: String) -> Result<String, PrError> {
    match head_file_contents.lines().next() {
        Some(line) => {
            if !line.contains('/') {
                return Err(PrError::Repo(
                    "Could not find current branch from git config".to_string(),
                ));
            }

            Ok(
                line.split('/').map(String::from).collect::<Vec<String>>()[2..].join("/")
            )
        }
        None => Err(PrError::Io(
            "Could not find git HEAD file".to_string(),
        )),
    }
}

#[test]
fn test_current_branch() {
    let hf = read_file(&Path::new("./tests/HEAD_A")).expect("test file");
    let actual = current_branch(hf);

    assert_eq!(Ok(String::from("test-branch")), actual);

    // it should handle branches with path names
    let actual = current_branch(String::from("ref: refs/heads/feat/test-branch"));
    assert_eq!(Ok(String::from("feat/test-branch")), actual);

    let actual = current_branch(String::from("ref: refs/heads/feat/nested//test-branch"));
    assert_eq!(Ok(String::from("feat/nested//test-branch")), actual);
}

#[test]
#[should_panic]
fn test_current_branch_errors() {
    // it should panic if file doesn't exist or is malformed
    let hf = read_file(&Path::new("./tests/HEAD_FILENOTFOUND")).expect("test file");
    current_branch(hf).unwrap();

    let hf = read_file(&Path::new("./tests/HEAD_BROKEN")).expect("test file");
    current_branch(hf).unwrap();
}

#[test]
fn test_branch() {
    // it should return a branch from the path to the git HEAD file
    assert_eq!(
        Ok(String::from("test-branch")),
        branch(&Path::new("./tests/HEAD_A"))
    );
    assert_eq!(
        Err(PrError::Io(
            "Cannot read .git HEAD file; No such file or directory (os error 2)".to_string()
        )),
        branch(&Path::new("./tests/HEAD_FILENOTFOUND"))
    );
    assert_eq!(
        Err(PrError::Repo(
            "Could not find current branch from git config".to_string(),
        )),
        branch(&Path::new("./tests/HEAD_BROKEN"))
    );
}

pub fn build_pr_msg(msg_path: Option<&str>) -> Result<PullRequestMsg, PrError> {
    let p = match msg_path {
        Some(p) => p,
        None => PR_EDITMSG_PATH,
    };
    let mut _title = String::new();

    // let pr_file: String = fs::read_to_string(p).expect("read test file");
    let pr_file: String = fs::read_to_string(p)?;
    let mut lines = pr_file.lines();

    // set the first line as title
    if let Some(t) = lines.next() {
        _title = String::from(t);
    } else {
        return Err(PrError::Repo("Unable to read title".to_string()));
    }

    let msg_body: String = lines
        .take_while(|line| !line.starts_with("// Requesting a pull to"))
        .collect();

    dbg!(&msg_body);

    Ok(PullRequestMsg {
        title: _title,
        body: msg_body,
    })
}

#[test]
fn test_build_message() -> Result<(), Box<dyn std::error::Error>> {
    // it should build a PullRequestMsg from file
    const PR_EDITMSG_PATH: &str = "./tests/PR_EDITMSG";
    let mut f = File::create(PR_EDITMSG_PATH)?;
    f.write_all(
        b"test title\n\nthis is a test msg body\n\n// Requesting a pull to master from feat",
    )?;

    let expected = PullRequestMsg {
        title: String::from("test title"),
        body: String::from("this is a test msg body"),
    };
    assert_eq!(Ok(expected), build_pr_msg(Some(PR_EDITMSG_PATH)));
    Ok(())
}

/// captures author, repo, and remote url from git config file
pub fn repo_config<'a>(text: &'a str, remote_match: &'a str) -> Result<RepoData<'a>, PrError> {
    let re = Regex::new(r#"\[remote\s+"(?P<origin>\w+)"\]\n\turl\s=\s(https?://|git@)github.com[:/]?(?P<author>[A-Za-z0-9_-]+)/(?P<repo>[A-Za-z0-9_-]+)"#).unwrap();

    // get only lines that match on the remote provided so that we
    // avoid confusion with forked remotes of the same name
    let match_lines = match re
        .captures(text)
        .filter(|c| c.name("origin").unwrap().as_str() == remote_match)
    {
        Some(m) => m,
        None => return Err(PrError::Repo("failed to read repo config".to_string())),
    };

    Ok(RepoData {
        author: match match_lines.get(3) {
            Some(s) => s.as_str(),
            None => {
                return Err(PrError::Repo(
                    "cannot find author data in config".to_string(),
                ))
            }
        },
        repo_name: match match_lines.get(4) {
            Some(s) => s.as_str(),
            None => {
                return Err(PrError::Repo(
                    "cannot find repo name data in config".to_string(),
                ))
            }
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
        Err(PrError::Repo("failed to read repo config".to_string())),
        repo_config(cfg_file, "upstream"),
        "Error on non existent remote"
    );
    assert_eq!(
        Err(PrError::Repo("failed to read repo config".to_string())),
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

#[derive(Debug, Deserialize)]
pub struct VcsApiResponseSuccess {
    pub html_url: String,
    pub number: u32
}

#[derive(Debug, Deserialize)]
struct VcsApiResponseError {
    pub message: String,
}

pub async fn fetch_api<'a>(
    repo_data: RepoData<'a>,
    token: &str,
    body: serde_json::Value,
) -> Result<VcsApiResponseSuccess, PrError> {
    let url = format!(
        "https://{}:{}@api.github.com/repos/{}/{}/pulls",
        &repo_data.author, &token, &repo_data.author, &repo_data.repo_name
    );

    let mut req = surf::post(&url).body_json(&body)?.await?; 

    if let http::status::StatusCode::CREATED  = req.status() {
        let response: VcsApiResponseSuccess = req.body_json().await?;
        dbg!(&response);
        Ok(response)
    } else {
        let response: VcsApiResponseError = req.body_json().await?;
        Err(PrError::Api(response.message.to_string()))
    }
}
