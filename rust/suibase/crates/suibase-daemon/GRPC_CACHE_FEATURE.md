# gRPC Cache Feature

## Executive Summary

This document outlines the client-side micro-caching system for the gRPC feature in Suibase. The design uses `foyer` for hybrid memory+disk caching of static content and `moka` for memory-only TTL and micro pools, leveraging each library's strengths.

## Cache Architecture

### Cache Pools - Adaptive Sizing

Automatic configuration based on system memory:

**Small Systems** (< 64 GB RAM):
- **Static Pool**: 150 MB memory + 1 GB disk
- **TTL Pool**: 100 MB memory
- **Micro Pool**: 10 MB memory
- **Total**: ~260 MB RAM + 1 GB disk

**Large Systems** (≥ 64 GB RAM):
- **Static Pool**: 500 MB memory + 5 GB disk
- **TTL Pool**: 300 MB memory
- **Micro Pool**: 20 MB memory
- **Total**: ~820 MB RAM + 5 GB disk

Note: System detection is automatic but can be overridden via `suibase.yaml`. The micro pool is intentionally small since items only live for 10ms.

### Storage Tier - Static Pool Only

**Managed by foyer**: The disk tier is automatically handled by foyer for the Static Pool
- Location: `~/suibase/workdirs/common/cache/grpc/*`
- Capacity: Auto-detected (Small: 1 GB, Large: 5 GB)
- Async I/O: Built into foyer, never blocks requests
- Admission control: S3-FIFO (recommended for our workload)
- Automatic promotion: Disk hits promoted to memory tier

### Eviction Algorithms

**Foyer (Static Pool)**: Using S3-FIFO (recommended)
- Excellent scan resistance for bulk operations
- Lower CPU overhead than TinyLFU
- Ideal for mixed workloads with immutable data
- Balances recency and frequency effectively

**Moka (TTL/Micro Pools)**: Window-TinyLFU
- Window Buffer: Small admission window (1% of cache)
- Frequency tracking via Count-Min Sketch
- Smart admission based on access patterns

## Pool-Specific Configuration

### Static Pool (Immutable Data) - Using Foyer
```rust
// Small system defaults (auto-detected, configurable)
foyer::CacheBuilder::new(150_000_000)  // 150MB memory
    .with_disk_cache(1_000_000_000)    // 1GB disk
    .with_admission_policy(AdmissionPolicy::S3Fifo)  // Best for our use case
    .build()
```
- **Key**: hash(method + params)
- **TTL**: None (infinite until evicted)
- **Storage**: Automatic hybrid memory+disk
- **Use Case**: GetObject, GetTransaction, GetPackage, etc.

### TTL Pool (Time-Sensitive Data) - Using Moka
```rust
// Small system defaults (auto-detected, configurable)
moka::Cache::builder()
    .max_capacity(100_000_000)  // 100MB
    .time_to_live(Duration::from_secs(60))
    .time_to_idle(Duration::from_secs(30))
    .build()
```
- **Key**: hash(method + params)
- **TTL**: Configurable 5-60 seconds
- **Storage**: Memory-only (expires too quickly for disk)
- **Use Case**: GetLatestCheckpoint, GetBalance, etc.

### Micro Pool (Request Deduplication) - Using Moka
```rust
// Small system defaults (auto-detected, configurable)
moka::Cache::builder()
    .max_capacity(10_000_000)   // 10MB (sufficient for 10ms TTL)
    .time_to_live(Duration::from_millis(10))
    .build()
```
- **Key**: hash(method + params + client_encoding) // Include client's accepted encoding
- **TTL**: 10ms base + transcoding time (if applicable)
- **Storage**: Memory-only, stores client-ready format
- **Use Case**: Dedup rapid identical requests from UI re-renders

## Design Rationale

### Library Selection
- **Foyer for Static Pool**: Purpose-built for hybrid memory+disk caching of immutable data
- **Moka for TTL/Micro Pools**: Excellent for memory-only caching with time-based expiry
- Each library used where it excels, minimizing custom code

### Micro-Cache Special Design
The micro-cache stores CLIENT-READY formats (post-transcoding) rather than original zstd because:
1. **Transcoding time > TTL**: With 80ms transcoding but 10ms TTL, normal caching would expire before transcoding completes
2. **Same client duplicates**: Buggy UIs make identical requests with same encoding requirements
3. **Immediate serving**: Pre-transcoded format eliminates the 80ms penalty on duplicate requests
4. **Extended TTL**: Cache entries live for 10ms + transcoding time to prevent mid-operation expiry

### Storage Strategy
- **Static Pool**: Hybrid memory+disk via foyer (immutable data benefits from persistence)
- **TTL Pool**: Memory-only (expires in 5-300s, disk would constantly churn)
- **Micro Pool**: Memory-only (10ms expiry makes disk I/O pointless)

## Collision Detection Strategy

### Problem
Using only a hash as a cache key could theoretically lead to hash collisions where different method+params combinations produce the same hash, potentially returning wrong data to users - a critical security issue for blockchain RPC.

### Solution
Store the original method_id and params in each cache entry for verification:

1. **Key Generation**: Use 128-bit hash of (method_id + params) as cache key
2. **Collision Check**: On cache hit, verify `entry.method_id == requested_method_id && entry.params == requested_params`
3. **Collision Handling**: If mismatch detected (extremely rare), log warning and treat as cache miss
4. **Storage Overhead**: ~302 bytes per entry (1 byte method_id + 1 byte encoding + ~300 bytes params)

### Performance Analysis
- **Hash Comparison**: Single CPU instruction (comparing 128-bit key)
- **Collision Check**: ~0.02 microseconds for 300-byte memcmp
- **Total Overhead**: Negligible (~0.02μs added to sub-millisecond cache hits)
- **Memory Trade-off**: 300 bytes params storage vs 100% security guarantee

### Optimization Details
- **Method as u8**: Instead of storing method names as strings (40+ bytes), use enum IDs (1 byte)
- **Encoding as u8**: Instead of "zstd"/"gzip" strings (24+ bytes), use enum (1 byte)
- **Params uncompressed**: Direct memcmp is faster than decompress+compare
- **Total savings**: 62 bytes per entry while maintaining collision detection

## Cache Operations

### Pool-Specific Storage Strategy

**Static & TTL Pools**:
- Store original zstd-compressed messages from servers
- Transcode on-demand when serving to clients
- Cache key: `hash(method + params)`
- Note: Same method may route to different pools based on params

**Micro Pool** (Special Case):
- Stores CLIENT-READY format (potentially transcoded)
- No transcoding on serve - already in correct format
- Cache key: `hash(method + params + client_encoding)`
- Rationale: Avoids 80ms transcoding penalty exceeding 10ms TTL

### Lookup Flow
```
1. Extract grpc-accept-encoding from client request headers
2. Determine cache type based on method + params (deterministic)
   - Note: Same method can map to different cache types based on params
   - Example: GetObject(id, version) -> Static, GetObject(id, null) -> TTL
3. Compute cache key from method + params (no compression info)
4. Direct lookup in the appropriate cache pool (no searching)
5. On cache hit:
   - Check if client supports cached compression format
   - Transcode if necessary (decompress/recompress)
   - Return transcoded response with appropriate grpc-encoding header
6. On cache miss:
   - Forward request to upstream server
   - Store response with original compression state
   - Preserve grpc-encoding header value
```

### Transcoding Logic

Since all Suibase-controlled servers use zstd compression, cached entries are always zstd-compressed. The cache handles transcoding for clients that don't support zstd:

```
Client Accept    | Action                      | Transcoding Penalty
-----------------|-----------------------------|--------------------------
zstd             | Serve as-is                 | None (0ms)
gzip             | zstd→gzip                   | 80ms/MB
deflate          | zstd→deflate                | 85ms/MB
none             | zstd→uncompressed           | 2ms/MB (decompress only)
```

**Performance Notes**:
- Transcoding happens on every request that needs it
- Most modern clients support zstd, reducing transcoding frequency
- Transcoding is CPU-bound but doesn't block other requests
- Consider: With cache TTLs (5-300s), lazy caching transcoded versions may not be worth the memory overhead

**Optimization Strategy**:
- Store as zstd for 20-25% better compression
- Lazy-cache gzip version after first transcoding
- Pre-transcode hot entries in background
- Encourage client zstd adoption via headers

### Memory Management

**Foyer (Static Pool)**:
- Automatically manages memory-to-disk promotion/eviction
- Async I/O ensures no request blocking
- Disk serves as extended cache for immutable data

**Moka (TTL/Micro Pools)**:
- Window-TinyLFU evicts least valuable items
- Each pool stays within configured limits
- No disk involvement needed

## Implementation Strategy

### Core Architecture

```rust
use foyer::{Cache as FoyerCache, CacheBuilder as FoyerBuilder};
use moka::Cache as MokaCache;

/// Method IDs for cache entries (instead of string names)
/// Each cache pool has <200 methods, so u8 is sufficient
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum CacheMethodId {
    // Static Pool Methods (0-99)
    GetObject = 0,
    GetTransaction = 1,
    GetCheckpoint = 2,
    GetPackage = 3,
    GetModule = 4,
    GetDatatype = 5,
    GetFunction = 6,
    // ... more static methods

    // TTL Pool Methods (100-199)
    GetLatestCheckpoint = 100,
    GetBalance = 101,
    GetValidators = 102,
    GetSystemState = 103,
    GetReferenceGasPrice = 104,
    // ... more TTL methods

    // Micro Pool Methods (200-255)
    // Each method must be explicitly listed (opt-in safety)
    GetEpoch = 200,
    GetProtocolConfig = 201,
    GetCommittee = 202,
    GetTotalSupply = 203,
    GetStakingPoolInfo = 204,
    GetValidatorAPY = 205,
    ResolveNameService = 206
}

/// Compression encoding as enum for efficiency
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum CompressionEncoding {
    None = 0,
    Zstd = 1,
    Gzip = 2,
    Deflate = 3,
    Snappy = 4,
    Brotli = 5,
}

/// Represents a cached gRPC message with collision detection
#[derive(Clone)]
pub struct CachedGrpcMessage {
    /// Complete gRPC wire format message (includes Compressed-Flag + Message-Length + Message)
    pub grpc_message: Vec<u8>,
    /// Compression algorithm (1 byte instead of String)
    pub encoding: CompressionEncoding,
    /// Method ID (1 byte instead of String)
    pub method_id: CacheMethodId,
    /// Original request parameters for collision detection (stored uncompressed)
    /// Vec already tracks length, no need for separate param_len field
    pub params: Vec<u8>,
}

// Memory overhead comparison:
// Before: method String (40 bytes) + encoding String (24 bytes) = 64 bytes
// After: method_id (1 byte) + encoding (1 byte) + params (~300 bytes) = ~302 bytes
// Savings: 62 bytes per entry (excluding params which are needed for collision detection)

pub struct GrpcCache {
    // Foyer for static content with built-in hybrid storage
    static_cache: FoyerCache<String, CachedGrpcMessage>,

    // Moka for time-sensitive content (memory-only)
    ttl_cache: MokaCache<String, CachedGrpcMessage>,

    // Moka for request deduplication (memory-only)
    micro_cache: MokaCache<String, CachedGrpcMessage>,
}

impl GrpcCache {
    pub async fn new() -> Result<Self> {
        // Auto-detect system size based on available memory
        let mut config = if system_memory_gb() >= 64 {
            CacheConfig::large_system()
        } else {
            CacheConfig::small_system()
        };

        // Apply user overrides from suibase.yaml grpc_cache section
        // Any field present in YAML overrides the auto-detected value
        config.apply_user_overrides_from_yaml()?;

        Ok(Self {
            // Foyer handles memory+disk transparently
            static_cache: FoyerBuilder::new(config.static_memory_mb * 1_000_000)
                .with_disk_cache(config.static_disk_gb * 1_000_000_000)
                .with_admission_policy(AdmissionPolicy::S3Fifo)
                .build()
                .await?,

            // Moka for time-sensitive content
            ttl_cache: MokaCache::builder()
                .max_capacity(config.ttl_memory_mb * 1_000_000)
                .time_to_live(Duration::from_secs(config.ttl_seconds))
                .build(),

            // Moka for deduplication
            micro_cache: MokaCache::builder()
                .max_capacity(config.micro_memory_mb * 1_000_000)
                .time_to_live(Duration::from_millis(config.micro_ttl_ms))
                .build(),
        })
    }

    /// Store a gRPC response in the appropriate cache pool
    pub async fn store(
        &self,
        method: &str,
        params: &[u8],
        grpc_wire_message: Vec<u8>,  // Already includes Compressed-Flag
        grpc_encoding: Option<&str>,  // e.g., "zstd"
        client_encoding: Option<&str>,  // Client's requested encoding for micro-cache
    ) -> Result<()> {
        let (cache_type, method_id) = Self::determine_cache_type_and_id(method);

        match cache_type {
            CacheType::Static | CacheType::TTL => {
                let method_id = method_id.ok_or("Method not cacheable")?;
                let encoding = Self::string_to_encoding(grpc_encoding);

                // Store original format (typically zstd from our servers)
                let key = Self::compute_key(method_id, params);
                let cached_msg = CachedGrpcMessage {
                    grpc_message: grpc_wire_message,
                    encoding,
                    method_id,
                    params: params.to_vec(), // Store for collision detection
                };

                match cache_type {
                    CacheType::Static => self.static_cache.insert(key, cached_msg).await,
                    CacheType::TTL => self.ttl_cache.insert(key, cached_msg).await,
                    _ => unreachable!(),
                }
            }
            CacheType::Micro => {
                // Micro-cache: all methods must have explicit IDs (opt-in safety)
                let method_id = method_id.ok_or("Micro-cache method missing ID")?;
                let key = Self::compute_micro_key(method_id, params, client_encoding);

                // Check if transcoding is needed
                let (final_message, final_encoding) =
                    if client_encoding != grpc_encoding {
                        // Transcode to client's format
                        let transcoded = Self::transcode_message(
                            &grpc_wire_message,
                            grpc_encoding,
                            client_encoding,
                        )?;
                        (transcoded, Self::string_to_encoding(client_encoding))
                    } else {
                        // Already in correct format
                        (grpc_wire_message, Self::string_to_encoding(grpc_encoding))
                    };

                let cached_msg = CachedGrpcMessage {
                    grpc_message: final_message,
                    encoding: final_encoding,
                    method_id,
                    params: params.to_vec(), // Store for collision detection
                };
                self.micro_cache.insert(key, cached_msg).await;
            }
            CacheType::NoCache => {
                // Don't cache real-time methods
                return Ok(());
            }
        }
        Ok(())
    }

    /// Retrieve and transcode a cached response for the client with collision detection
    pub async fn get(
        &self,
        method: &str,
        params: &[u8],
        client_accept_encoding: &[String],
    ) -> Option<Vec<u8>> {
        let (cache_type, method_id) = Self::determine_cache_type_and_id(method);

        match cache_type {
            CacheType::Static | CacheType::TTL => {
                let method_id = method_id?; // These cache types require a valid method_id

                // Standard caching: key without encoding, transcode on serve
                let key = Self::compute_key(method_id, params);
                let entry = match cache_type {
                    CacheType::Static => self.static_cache.get(&key).await?,
                    CacheType::TTL => self.ttl_cache.get(&key).await?,
                    _ => unreachable!(),
                };

                // Collision detection: verify method_id and params match exactly
                if entry.method_id != method_id || entry.params != params {
                    // Hash collision detected (extremely rare with 128-bit hash)
                    warn!("Cache hash collision detected for method {:?}!", method_id);
                    return None; // Treat as cache miss
                }

                entry.serve_to_client(client_accept_encoding).ok()
            }
            CacheType::Micro => {
                // Micro-cache: all methods must have explicit IDs (opt-in safety)
                let method_id = method_id?;

                // Micro-cache: key includes encoding, serves client-ready format
                let client_encoding = client_accept_encoding.first().map(|s| s.as_str());
                let key = Self::compute_micro_key(method_id, params, client_encoding);
                let entry = self.micro_cache.get(&key).await?;

                // Collision detection for micro cache
                if entry.method_id != method_id || entry.params != params {
                    warn!("Micro-cache hash collision detected!");
                    return None;
                }

                Some(entry.grpc_message.clone())
            }
            CacheType::NoCache => None,
        }
    }

    fn compute_key(method_id: CacheMethodId, params: &[u8]) -> String {
        // Use xxHash for stable, fast hashing (critical for disk-persistent cache)
        // xxHash is deterministic across builds, unlike DefaultHasher
        use twox_hash::XxHash64;
        use std::hash::{Hash, Hasher};

        let mut hasher = XxHash64::with_seed(0); // Fixed seed for stability
        (method_id as u8).hash(&mut hasher);
        params.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn compute_micro_key(method_id: CacheMethodId, params: &[u8], client_encoding: Option<&str>) -> String {
        // Micro-cache key includes client's encoding preference
        // Uses same stable hasher for consistency (even though micro-cache is memory-only)
        use twox_hash::XxHash64;
        use std::hash::{Hash, Hasher};

        let mut hasher = XxHash64::with_seed(0);
        (method_id as u8).hash(&mut hasher);
        params.hash(&mut hasher);
        client_encoding.unwrap_or("none").hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Convert string encoding to enum
    fn string_to_encoding(encoding: Option<&str>) -> CompressionEncoding {
        match encoding {
            Some("zstd") => CompressionEncoding::Zstd,
            Some("gzip") => CompressionEncoding::Gzip,
            Some("deflate") => CompressionEncoding::Deflate,
            Some("snappy") => CompressionEncoding::Snappy,
            Some("brotli") => CompressionEncoding::Brotli,
            _ => CompressionEncoding::None,
        }
    }

    /// Convert enum encoding back to string
    fn encoding_to_string(encoding: CompressionEncoding) -> Option<&'static str> {
        match encoding {
            CompressionEncoding::Zstd => Some("zstd"),
            CompressionEncoding::Gzip => Some("gzip"),
            CompressionEncoding::Deflate => Some("deflate"),
            CompressionEncoding::Snappy => Some("snappy"),
            CompressionEncoding::Brotli => Some("brotli"),
            CompressionEncoding::None => None,
        }
    }

    /// Deterministically select cache pool and method ID based on method name
    /// SAFETY: Uses "opt-in" approach - only explicitly listed methods are cached
    /// NOTE: In a full implementation, this should also consider params since some
    /// methods map to different cache types based on their parameters.
    /// Example: GetObject(version) -> Static, GetObject(latest) -> TTL
    fn determine_cache_type_and_id(method: &str) -> (CacheType, Option<CacheMethodId>) {
        match method {
            // Static methods (immutable data)
            // TODO: GetObject should check params - only static if version specified
            "GetObject" => (CacheType::Static, Some(CacheMethodId::GetObject)),
            "GetTransaction" => (CacheType::Static, Some(CacheMethodId::GetTransaction)),
            "GetCheckpoint" => (CacheType::Static, Some(CacheMethodId::GetCheckpoint)),
            "GetPackage" => (CacheType::Static, Some(CacheMethodId::GetPackage)),
            "GetModule" => (CacheType::Static, Some(CacheMethodId::GetModule)),
            "GetDatatype" => (CacheType::Static, Some(CacheMethodId::GetDatatype)),
            "GetFunction" => (CacheType::Static, Some(CacheMethodId::GetFunction)),

            // TTL methods (time-sensitive but cacheable)
            "GetLatestCheckpoint" => (CacheType::TTL, Some(CacheMethodId::GetLatestCheckpoint)),
            "GetBalance" => (CacheType::TTL, Some(CacheMethodId::GetBalance)),
            "GetValidators" => (CacheType::TTL, Some(CacheMethodId::GetValidators)),
            "GetSystemState" => (CacheType::TTL, Some(CacheMethodId::GetSystemState)),
            "GetReferenceGasPrice" => (CacheType::TTL, Some(CacheMethodId::GetReferenceGasPrice)),

            // Micro-cache methods (very short TTL for deduplication)
            // These are read-only methods that change frequently but benefit from
            // deduplication of rapid identical requests (e.g., UI polling)
            // Note: Some of these may move to TTL pool based on actual usage patterns
            "GetEpoch" => (CacheType::Micro, Some(CacheMethodId::GetEpoch)),
            "GetProtocolConfig" => (CacheType::Micro, Some(CacheMethodId::GetProtocolConfig)),
            "GetCommittee" => (CacheType::Micro, Some(CacheMethodId::GetCommittee)),
            "GetTotalSupply" => (CacheType::Micro, Some(CacheMethodId::GetTotalSupply)),
            "GetStakingPoolInfo" => (CacheType::Micro, Some(CacheMethodId::GetStakingPoolInfo)),
            "GetValidatorAPY" => (CacheType::Micro, Some(CacheMethodId::GetValidatorAPY)),
            "ResolveNameService" => (CacheType::Micro, Some(CacheMethodId::ResolveNameService)),

            // DEFAULT: Unknown methods are NEVER cached (safety first)
            // This prevents accidentally caching new mutating methods
            _ => (CacheType::NoCache, None),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum CacheType {
    Static,   // Immutable data - infinite TTL
    TTL,      // Time-sensitive - configurable TTL (5-300s)
    Micro,    // Request deduplication - very short TTL (10ms)
    NoCache,  // Real-time data - never cached
}
```

### Implementation Requirements

#### Safety-First: Opt-In Caching
The cache uses an "opt-in" approach for safety. Only methods explicitly classified in `determine_cache_type_and_id` are cached. All unknown methods default to `NoCache`.

This prevents catastrophic bugs where:
- New mutating methods are accidentally cached
- Protocol updates introduce methods with side effects
- Unknown methods corrupt the cache or cause incorrect behavior

**Never change the default from `NoCache` to any form of caching.** Each method must be individually reviewed and explicitly added to the appropriate cache pool.

#### Stable Hashing for Persistent Cache
The Static Pool uses foyer with disk persistence. The cache keys MUST use a stable, deterministic hashing algorithm that produces identical results across:
- Different compilations of the same code
- Different versions of the Rust compiler
- Different runs of the program
- System restarts

For this reason, we use `twox_hash::XxHash64` with a fixed seed instead of `std::collections::hash_map::DefaultHasher`, which is explicitly not stable across builds.


## Integration with gRPC Feature

This caching system integrates with the main gRPC feature by:

1. **Method Classification**: Uses the Static/Cached/Real-Time classification from GRPC_FEATURE.md
2. **Request Interception**: Sits between the client and the routing layer
3. **Transparent Operation**: No changes required to client applications


## Configuration

The cache automatically detects system size (< 64GB RAM = small, ≥ 64GB RAM = large) and applies appropriate defaults. Any value specified in `suibase.yaml` overrides the corresponding auto-detected default:

```yaml
grpc_cache:
  enabled: true
  # Any value defined here overrides the auto-detected default.
  # If a value is omitted, the auto-detected default is used.

  # Example: Override only memory sizes, keep auto-detected disk and TTL values
  static_memory_mb: 250      # Overrides auto-detected (Small: 150MB, Large: 500MB)
  ttl_memory_mb: 150        # Overrides auto-detected (Small: 100MB, Large: 300MB)
  # static_disk_gb not specified, uses auto-detected (Small: 1GB, Large: 5GB)
  # ttl_seconds not specified, uses auto-detected (Both: 60s)
  # micro_memory_mb not specified, uses auto-detected (Small: 10MB, Large: 20MB)
  # micro_ttl_ms not specified, uses auto-detected (Both: 10ms)
```

## Performance Targets

- Cache hit ratio: >80% for repeated requests
- Lookup latency: <1ms for memory tier, <5ms for storage tier
- Memory overhead: <5% for metadata and indices
- CPU overhead: <2% for cache management
- **Critical**: Cache operations must NEVER increase request latency
- Disk writes are purely opportunistic background operations
- Cache misses should be no slower than without cache

## Compression Handling

### Native gRPC Compression Support

The cache leverages gRPC's built-in compression protocol:

**Wire Format** (stored in cache):
- **Compressed-Flag** (1 byte): 0=uncompressed, 1=compressed
- **Message-Length** (4 bytes): Size of the message in big-endian
- **Message**: The actual message bytes (possibly compressed)

**HTTP/2 Headers** (stored separately):
- `grpc-encoding`: Compression algorithm used (e.g., "zstd", "gzip")
- `grpc-accept-encoding`: Algorithms the client supports

### Storage Strategy

**Default Compression**: zstd
- All Suibase-controlled servers will use zstd compression by default
- Both in-memory and disk cache entries stored as zstd-compressed
- 20-25% better compression ratio than gzip
- 3-10x faster decompression speeds

**Cache Storage**:
- All cached entries are zstd-compressed (from Suibase servers)
- Preserves original `grpc-encoding` header ("zstd")
- Cache key remains simple: `hash(method + params)` (no compression info)
- Stores raw gRPC messages with their Compressed-Flag intact

**Design Note**: While the cache implementation can technically accept any compression format for flexibility, in practice all entries are zstd since that's what our servers produce

### Smart Transcoding

The cache provides universal transcoding between any compression formats. Since most cached entries will be zstd-compressed (from Suibase servers), the cache optimizes for zstd→gzip transcoding:

```rust
pub struct CachedGrpcMessage {
    // Complete gRPC wire format (Compressed-Flag + Message-Length + Message)
    grpc_message: Vec<u8>,
    // Compression algorithm from grpc-encoding header (usually "zstd")
    encoding: Option<String>,
}

impl CachedGrpcMessage {
    pub fn serve_to_client(
        &self,
        client_accept_encoding: &[String],
    ) -> Result<Vec<u8>> {
        // Fast path: client supports our stored format (usually zstd)
        if let Some(encoding_str) = GrpcCache::encoding_to_string(self.encoding) {
            if client_accept_encoding.iter().any(|e| e == encoding_str) {
                return Ok(self.grpc_message.clone());
            }
        }

        // Extract Compressed-Flag from wire format to check if transcoding needed
        let is_compressed = self.grpc_message[0] == 1;

        // Transcoding path: convert between any formats
        // Most common: zstd → gzip for legacy clients
        let stored_encoding = GrpcCache::encoding_to_string(self.encoding);
        let target_encoding = client_accept_encoding.first().map(|s| s.as_str());

        match (stored_encoding, target_encoding) {
            (Some("zstd"), Some("gzip")) => {
                // Extract the actual message bytes (skip Compressed-Flag and Message-Length)
                let message_bytes = &self.grpc_message[5..];
                // Transcode: zstd decompression is very fast (~500MB/s)
                let uncompressed = zstd::decode(message_bytes)?;
                let gzip_compressed = gzip::encode(&uncompressed)?;
                // Rebuild gRPC wire format with new compressed message
                let mut grpc_gzip = vec![1]; // Compressed-Flag = 1
                grpc_gzip.extend_from_slice(&(gzip_compressed.len() as u32).to_be_bytes());
                grpc_gzip.extend_from_slice(&gzip_compressed);
                Ok(grpc_gzip)
            }
            // Handle any other format combinations
            (Some(from), Some(to)) if from != to => {
                self.transcode(from, to)
            }
            _ => Ok(self.grpc_message.clone())
        }
    }
}
```

### Compression Benefits

1. **Storage Efficiency**: zstd compression provides 20-25% better ratio than gzip
2. **Protocol Compliance**: Uses standard gRPC compression negotiation
3. **Future-Proof**: zstd is becoming the industry standard for gRPC
4. **Flexible Serving**: Universal transcoding between any compression formats
5. **Network Efficiency**: Reduced bandwidth with better compression ratios
6. **Performance**: zstd decompresses 3-10x faster than gzip
7. **Cost Savings**: 20-30% reduction in memory/disk usage vs gzip

### Implementation Notes

- **Default Server Compression**: All Suibase servers configured to use zstd
- **Storage Format**: Cache stores complete gRPC wire format messages (with Compressed-Flag)
- **Compression contexts**: Per-message (not maintained across boundaries)
- **Supported algorithms**: zstd (preferred), gzip, deflate, snappy, brotli, none
- **Lazy Evaluation**: Decompression/transcoding only when needed
- **Transcoding Cache**: Frequently accessed entries cache alternative encodings
- **Cache hit rates**: Unaffected by compression differences (key excludes compression)
- **Transcoding Location**: Happens in the serving path, not storage path
- **Performance Priority**: Optimized for zstd→gzip transcoding (most common scenario)

## Future Enhancements

- Cache warming on startup from frequently accessed data
- Machine learning-based prediction for pre-fetching
- Advanced compression algorithms (zstd, brotli) for disk tier