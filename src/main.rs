use async_std::task;

// #[runtime::main]
// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
fn main() -> Result<(), surf::Exception> {
    task::block_on(async {
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
