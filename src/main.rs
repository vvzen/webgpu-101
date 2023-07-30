mod core;

#[pollster::main]
async fn main() {
    println!("Hello, WebGPU world!");

    core::run().await;
}
