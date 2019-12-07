use std::fs::File;
use std::io::{self, Read, Write, BufReader};
use async_std::task;

use promptly::{prompt};


fn prompt_username() -> String {
    let mut res = String::new(); 

    writeln!(io::stdout(), "Username for 'https://github.com': ").expect("write stdout");
    io::stdin().read_line(&mut res).expect("read stdin");
    res
}

// #[runtime::main]
// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
fn main() -> std::io::Result<()> {

    let mut f = File::open(".git/config")?;
    let mut reader = BufReader::new(f);
    let mut body = String::new();

    reader.read_to_string(&mut body)?;

    for lin in body.lines() {
        println!("line: {}", lin);
    }


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
