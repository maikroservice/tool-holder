use tool_holder::config::load_all;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let configs = load_all("tools")?;
    println!("Loaded {} tool config(s)", configs.len());
    Ok(())
}
