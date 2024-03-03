use std::env;

#[allow(warnings)]
fn main() {
    let path = env::current_dir().unwrap();
    println!("The current directory is {}", path.display());
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_test_writer()
        .init();

    api::main();
}
