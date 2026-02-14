use cudarc::driver::CudaContext;

fn main() {
    println!("Testing CUDA GPU detection...");
    
    // Check device count
    match CudaContext::device_count() {
        Ok(count) => {
            println!("✅ Found {} CUDA device(s)", count);
            
            if count > 0 {
                // Try to initialize first device
                match CudaContext::new(0) {
                    Ok(ctx) => {
                        println!("✅ Successfully initialized device 0");
                        println!("   Device: {:?}", ctx);
                    }
                    Err(e) => {
                        println!("❌ Failed to initialize device 0: {:?}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to get device count: {:?}", e);
        }
    }
}
