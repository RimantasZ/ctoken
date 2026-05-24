use anyhow::Result;

fn main() -> Result<()> {
    println!("Hello from sample!");
    let items = vec![1, 2, 3, 4, 5];
    let sum: i32 = items.iter().sum();
    println!("Sum: {}", sum);
    Ok(())
}
