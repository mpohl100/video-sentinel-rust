#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    if let Err(error) = video_sentinel::ros2_node::run_from_cli().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
