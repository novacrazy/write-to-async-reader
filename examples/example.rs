use std::io;

use serde::Serialize;

use write_to_async_reader::Reader;

#[derive(Debug, Serialize)]
struct Test {
    x: &'static str,
    a: i32,
    b: f32,
    c: [i16; 4],
    d: Option<Box<Test>>,
}

use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let value = Test {
        x: "Hello, World!",
        a: 42,
        b: 23.235235124,
        c: [1, 2, 3, 4],
        d: Some(Box::new(Test {
            x: "Hello again",
            a: 23,
            b: 3.14159,
            c: [3, 5, 8, 13],
            d: None,
        })),
    };

    let mut rdr = Reader::new(|w| {
        serde_json::to_writer(w, &value)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    });

    let mut out = String::new();

    rdr.read_to_string(&mut out).await?;

    println!("Out: {}", out);

    Ok(())
}
