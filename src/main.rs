use std::env;
use std::fs;
use std::fs::{OpenOptions};
use std::io::{self};
use std::path::Path;
use std::process::Command;

use async_std::task;
use regex::Regex;

extern crate clap;
use clap::{App, Arg};

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

#[test]
fn test_current_branch() {
    let hf = head_file(&Path::new("./tests/HEAD_A")).expect("test file");
    let actual = current_branch(hf);

    assert_eq!(Some(String::from("test-branch")), actual)
}

fn get_remote(text: &str) -> Result<Vec<&str>, ()> {
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

fn launch_editor() -> Result<(), ()> {
    let editor = env::var("GIT_EDITOR").expect("no $GIT_EDITOR set");
    let pr_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(".git/PR_EDITMSG");
    dbg!(pr_file);

    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("\"{} ./.git/PR_EDITMSG\"", &editor))
        .spawn()
        .expect("open editor");
    // writeln!(output.stdout, "{}", "butttts");
    dbg!(output);
    Ok(())
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

    dbg!(remote);
    dbg!(target);

    let git_head = head_file(&Path::new("./.git/HEAD")).expect("git HEAD");
    let br = current_branch(git_head);
    let token = env::var("GITHUB_TOKEN").expect("required GITHUB_TOKEN");

    dbg!(token);
    dbg!(br);

    launch_editor().expect("launch editor");

    Ok(())
}

fn xfetch_api(uname: &str, password: &str, repo: &str) -> Result<(), surf::Exception> {
    task::block_on(async {
        let url = format!(
            "https://{}:{}@api.github.com/repos/{}/{}/pulls?sort=created",
            &uname, &password, &uname, &repo
        );
        dbg!(&url);
        let res: String = surf::get(url).recv_string().await?;
        println!("{}", res);
        Ok::<(), surf::Exception>(())
    })
}
// The need for Ok with turbofish is explained here
// https://rust-lang.github.io/async-book/07_workarounds/03_err_in_async_blocks.html
// fn main() -> Result<(), surf::Exception> {
//     // femme::start(log::LevelFilter::Info)?;

//     task::block_on(async {
//         let uri = "https://httpbin.org/get";
//         let string: String = surf::get(uri).recv_string().await?;
//         println!("{}", string);
//         Ok::<(), surf::Exception>(())
//     })
// }
