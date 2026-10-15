async fn fetch(url: &str) -> Result<String, Error> { let r = client.get(url).send().await?; Ok(r) }
fn main() {
    let f = async move { fetch("x").await };
    let b: Box<dyn Fn()> = Box::new(|| ());
    let r#try = 1;
}
