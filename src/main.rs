#[runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut res = surf::get("https://httpbin.org/get").await?;
    dbg!(res.body_string().await?);
    Ok(())
}
