// async/await + ? operator coverage.

pub async fn fetch(url: &str) -> Result<String, std::io::Error> {
    let s = read_string(url).await?;
    Ok(s)
}

async fn read_string(_url: &str) -> Result<String, std::io::Error> {
    Ok(String::new())
}
