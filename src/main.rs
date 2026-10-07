use futures::{stream, StreamExt};
use std::{fs, io::Write, time::Duration};

async fn check(c: &reqwest::Client, name: &str) -> Option<bool> {
    let url = format!("https://steamcommunity.com/id/{name}");
    for try_n in 1..=5u64 {
        match c.get(&url).send().await {
            Ok(r) if r.status().as_u16() == 429 => {
                tokio::time::sleep(Duration::from_secs(30 * try_n)).await;
            }
            Ok(r) => {
                let t = r.text().await.ok()?;
                return Some(t.contains("The specified profile could not be found"));
            }
            Err(_) => tokio::time::sleep(Duration::from_secs(2)).await,
        }
    }
    None
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).map(String::as_str).unwrap_or("words.txt");
    let conc: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);

    let mut words: Vec<String> = fs::read_to_string(path)
        .expect("cannot read wordlist")
        .lines()
        .map(|l| l.trim().to_lowercase())
        .filter(|w| (3..=32).contains(&w.len()) && w.bytes().all(|b| b.is_ascii_lowercase()))
        .collect();
    words.sort();
    words.dedup();
    eprintln!("checking {} words", words.len());

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let mut out = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("available.txt")
        .unwrap();

    let mut results = stream::iter(words)
        .map(|w| {
            let c = client.clone();
            async move {
                let r = check(&c, &w).await;
                (w, r)
            }
        })
        .buffer_unordered(conc);

    while let Some((w, r)) = results.next().await {
        if r == Some(true) {
            println!("{w}");
            writeln!(out, "{w}").unwrap();
        }
    }
}
