use std::env;
use std::fmt::Error;
use std::fs;
use std::fs::{File};
use std::io::{self, Write};
use std::path::Path;

use async_std::task;
use regex::Regex;

fn branch_err() -> Error {
    Error
}

fn head_file(head_file: &Path) -> Result<String, io::Error> {
    fs::read_to_string(head_file)
}

/// read over the .git/HEAD file to get current branch
fn current_branch(head_file_contents: String) -> Option<String> {
    match head_file_contents.lines().next() {
        Some(line) => line.split('/').map(String::from).collect::<Vec<String>>().pop(),
        _ => None
    }
}

#[test]
fn test_current_branch() {
    let hf = head_file(&Path::new("./tests/HEAD_A")).expect("test file");
    let actual = current_branch(hf);

    assert_eq!(Some(String::from("test-branch")), actual)
}


fn get_remote(text: &str) -> Result<Vec<&str>, ()>  {
    let re = Regex::new(r#"\[remote\s+"(?P<origin>\w+)"\]\n\turl\s=\s(https?://|git@)github.com[:/]?(?P<author>[A-Za-z0-9_]+)/(?P<repo>[A-Za-z0-9_])"#).unwrap();

    let mut v: Vec<&str> = Vec::new();

    for caps in re.captures_iter(text) {
        match &caps.name("author") {
            Some(m) => v.push(m.as_str()),
            None => ()
        }

        match &caps.name("repo") {
            Some(m) => v.push(m.as_str()),
            None => ()
        }

        match &caps.name("origin") {
            Some(m) => v.push(m.as_str()),
            None => ()
        }

    }

    Ok(v)
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let remote = match args.len() {
        2 => "origin",
        3 => &args[2],
        _ => panic!("Incorrect args")
    };

    let git_head = head_file(&Path::new("./.git/HEAD")).expect("git HEAD");
    let br = current_branch(git_head);
    let token = env::var("GITHUB_TOKEN").expect("required GITHUB_TOKEN");

    dbg!(token);
    // reader.read_to_string(&mut body)?;

    // let target_remote = "origin";
    // let mut rmt = "wrong";

    // let caps = get_remote(body.as_str());

    // for lin in body.lines() {
    //     println!("line: {}", lin);
    //     rmt = get_remote(target_remote);
    // }
    // match caps {
        // Ok(c) => println!("remote {:?}", c[0]),
        // _ => println!("fn")
    // }

    Ok(())
}

fn fetch_api(uname: &str, password: &str, repo: &str) -> Result<(), surf::Exception> {
    task::block_on(async {
        let url = format!("https://{}:{}@api.github.com/repos/{}/{}/pulls?sort=created", &uname, &password, &uname, &repo);
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
