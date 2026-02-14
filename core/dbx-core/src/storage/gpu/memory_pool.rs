//! GPU Memory Pool for efficient allocation/deallocation

#[cfg(feature = "gpu")]
mod gpu_pool {
    use cudarc::driver::{CudaContext, CudaSlice};
    use std::collections::{HashMap, VecDeque};
    use std::sync::{Arc, Mutex};

    /// Memory pool bucket for a specific size range
    pub(super) struct MemoryBucket<T: Clone> {
        /// Free buffers ready for reuse
        free_buffers: VecDeque<CudaSlice<T>>,
        /// Maximum number of buffers to keep
        max_buffers: usize,
        /// Buffer size for this bucket
        buffer_size: usize,
    }

    impl<T: Clone + cudarc::driver::DeviceRepr> MemoryBucket<T> {
        pub(super) fn new(buffer_size: usize, max_buffers: usize) -> Self {
            Self {
                free_buffers: VecDeque::new(),
                max_buffers,
                buffer_size,
            }
        }

        /// Try to get a buffer from the free list
        pub(super) fn try_alloc(&mut self) -> Option<CudaSlice<T>> {
            self.free_buffers.pop_front()
        }

        /// Return a buffer to the free list (if not full)
        pub(super) fn free(&mut self, buffer: CudaSlice<T>) {
            if self.free_buffers.len() < self.max_buffers {
                self.free_buffers.push_back(buffer);
            }
            // Otherwise, buffer is dropped and freed
        }

        /// Clear all cached buffers
        pub(super) fn clear(&mut self) {
            self.free_buffers.clear();
        }
    }

    /// GPU Memory Pool - manages reusable GPU memory buffers
    pub struct GpuMemoryPool {
        device: Arc<CudaContext>,
        /// Buckets for i32 buffers (key = size)
        i32_buckets: Arc<Mutex<HashMap<usize, MemoryBucket<i32>>>>,
        /// Buckets for i64 buffers (key = size)
        i64_buckets: Arc<Mutex<HashMap<usize, MemoryBucket<i64>>>>,
        /// Buckets for f32 buffers (key = size)
        f32_buckets: Arc<Mutex<HashMap<usize, MemoryBucket<f32>>>>,
        /// Maximum total memory to cache (bytes)
        max_cache_bytes: usize,
        /// Current cached memory (bytes)
        cached_bytes: Arc<Mutex<usize>>,
    }

    impl GpuMemoryPool {
        /// Create a new memory pool
        pub fn new(device: Arc<CudaContext>, max_cache_mb: usize) -> Self {
            Self {
                device,
                i32_buckets: Arc::new(Mutex::new(HashMap::new())),
                i64_buckets: Arc::new(Mutex::new(HashMap::new())),
                f32_buckets: Arc::new(Mutex::new(HashMap::new())),
                max_cache_bytes: max_cache_mb * 1024 * 1024,
                cached_bytes: Arc::new(Mutex::new(0)),
            }
        }

        /// Round up size to next bucket size
        fn round_to_bucket_size(size: usize) -> usize {
            const BUCKET_SIZES: &[usize] = &[
                256,     // 256 elements
                1024,    // 1K
                4096,    // 4K
                16384,   // 16K
                65536,   // 64K
                262144,  // 256K
                1048576, // 1M
            ];

            for &bucket_size in BUCKET_SIZES {
                if size <= bucket_size {
                    return bucket_size;
                }
            }

            // For very large sizes, round up to nearest 1M
            ((size + 1048575) / 1048576) * 1048576
        }

        /// Allocate i32 buffer (from pool or new)
        pub fn alloc_i32(
            &self,
            size: usize,
        ) -> Result<CudaSlice<i32>, cudarc::driver::DriverError> {
            let bucket_size = Self::round_to_bucket_size(size);

            // Try to get from pool
            {
                let mut buckets = self.i32_buckets.lock().unwrap();
                if let Some(bucket) = buckets.get_mut(&bucket_size) {
                    if let Some(mut buffer) = bucket.try_alloc() {
                        // Update cached bytes
                        let mut cached = self.cached_bytes.lock().unwrap();
                        *cached = cached.saturating_sub(bucket_size * std::mem::size_of::<i32>());

                        // Resize if needed
                        if buffer.len() != size {
                            buffer = self.device.default_stream().alloc_zeros::<i32>(size)?;
                        }
                        return Ok(buffer);
                    }
                }
            }

            // Allocate new buffer
            self.device.default_stream().alloc_zeros::<i32>(size)
        }

        /// Allocate i64 buffer (from pool or new)
        pub fn alloc_i64(
            &self,
            size: usize,
        ) -> Result<CudaSlice<i64>, cudarc::driver::DriverError> {
            let bucket_size = Self::round_to_bucket_size(size);

            // Try to get from pool
            {
                let mut buckets = self.i64_buckets.lock().unwrap();
                if let Some(bucket) = buckets.get_mut(&bucket_size) {
                    if let Some(mut buffer) = bucket.try_alloc() {
                        // Update cached bytes
                        let mut cached = self.cached_bytes.lock().unwrap();
                        *cached = cached.saturating_sub(bucket_size * std::mem::size_of::<i64>());

                        // Resize if needed
                        if buffer.len() != size {
                            buffer = self.device.default_stream().alloc_zeros::<i64>(size)?;
                        }
                        return Ok(buffer);
                    }
                }
            }

            // Allocate new buffer
            self.device.default_stream().alloc_zeros::<i64>(size)
        }

        /// Allocate f32 buffer (from pool or new)
        pub fn alloc_f32(
            &self,
            size: usize,
        ) -> Result<CudaSlice<f32>, cudarc::driver::DriverError> {
            let bucket_size = Self::round_to_bucket_size(size);

            // Try to get from pool
            {
                let mut buckets = self.f32_buckets.lock().unwrap();
                if let Some(bucket) = buckets.get_mut(&bucket_size) {
                    if let Some(mut buffer) = bucket.try_alloc() {
                        // Update cached bytes
                        let mut cached = self.cached_bytes.lock().unwrap();
                        *cached = cached.saturating_sub(bucket_size * std::mem::size_of::<f32>());

                        // Resize if needed
                        if buffer.len() != size {
                            buffer = self.device.default_stream().alloc_zeros::<f32>(size)?;
                        }
                        return Ok(buffer);
                    }
                }
            }

            // Allocate new buffer
            self.device.default_stream().alloc_zeros::<f32>(size)
        }

        /// Return i32 buffer to pool
        pub fn free_i32(&self, buffer: CudaSlice<i32>) {
            let size = buffer.len();
            let bucket_size = Self::round_to_bucket_size(size);
            let byte_size = bucket_size * std::mem::size_of::<i32>();

            // Check if we have room in cache
            {
                let cached = self.cached_bytes.lock().unwrap();
                if *cached + byte_size > self.max_cache_bytes {
                    // Cache full, drop buffer
                    return;
                }
            }

            // Add to pool
            let mut buckets = self.i32_buckets.lock().unwrap();
            let bucket = buckets.entry(bucket_size).or_insert_with(|| {
                MemoryBucket::new(bucket_size, 8) // Max 8 buffers per bucket
            });
            bucket.free(buffer);

            // Update cached bytes
            let mut cached = self.cached_bytes.lock().unwrap();
            *cached += byte_size;
        }

        /// Return i64 buffer to pool
        pub fn free_i64(&self, buffer: CudaSlice<i64>) {
            let size = buffer.len();
            let bucket_size = Self::round_to_bucket_size(size);
            let byte_size = bucket_size * std::mem::size_of::<i64>();

            // Check if we have room in cache
            {
                let cached = self.cached_bytes.lock().unwrap();
                if *cached + byte_size > self.max_cache_bytes {
                    return;
                }
            }

            // Add to pool
            let mut buckets = self.i64_buckets.lock().unwrap();
            let bucket = buckets
                .entry(bucket_size)
                .or_insert_with(|| MemoryBucket::new(bucket_size, 8));
            bucket.free(buffer);

            // Update cached bytes
            let mut cached = self.cached_bytes.lock().unwrap();
            *cached += byte_size;
        }

        /// Return f32 buffer to pool
        pub fn free_f32(&self, buffer: CudaSlice<f32>) {
            let size = buffer.len();
            let bucket_size = Self::round_to_bucket_size(size);
            let byte_size = bucket_size * std::mem::size_of::<f32>();

            // Check if we have room in cache
            {
                let cached = self.cached_bytes.lock().unwrap();
                if *cached + byte_size > self.max_cache_bytes {
                    return;
                }
            }

            // Add to pool
            let mut buckets = self.f32_buckets.lock().unwrap();
            let bucket = buckets
                .entry(bucket_size)
                .or_insert_with(|| MemoryBucket::new(bucket_size, 8));
            bucket.free(buffer);

            // Update cached bytes
            let mut cached = self.cached_bytes.lock().unwrap();
            *cached += byte_size;
        }

        /// Clear all cached buffers
        pub fn clear(&self) {
            self.i32_buckets.lock().unwrap().clear();
            self.i64_buckets.lock().unwrap().clear();
            self.f32_buckets.lock().unwrap().clear();
            *self.cached_bytes.lock().unwrap() = 0;
        }

        /// Get current cached memory size (bytes)
        pub fn cached_bytes(&self) -> usize {
            *self.cached_bytes.lock().unwrap()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_bucket_size_rounding() {
            assert_eq!(GpuMemoryPool::round_to_bucket_size(100), 256);
            assert_eq!(GpuMemoryPool::round_to_bucket_size(256), 256);
            assert_eq!(GpuMemoryPool::round_to_bucket_size(257), 1024);
            assert_eq!(GpuMemoryPool::round_to_bucket_size(65536), 65536);
            assert_eq!(GpuMemoryPool::round_to_bucket_size(100000), 262144);
        }
    }
}

#[cfg(feature = "gpu")]
pub use gpu_pool::GpuMemoryPool;
