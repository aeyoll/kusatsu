// Backend server placeholder - will be implemented in Phase 2

use kusatsu_backend::{error::Result, run_server};

#[tokio::main]
async fn main() -> Result<()> {
    run_server().await
}
