# gRPC Feature

## Executive Summary

This document outlines the architecture for adding gRPC HTTP/2 support to Suibase users.

The system supports tiered services (free/pro) and intelligent load-balancing.

## Architecture Goals

- Application(s) connects to a local suibase-daemon port. The daemon select and forward to the best suited backend server to maintain low latency and resiliency. For the application: it just works as if connecting to a single gRPC server.

- For most users, there is no configuration needed. If the user choose to fund a shared custody account, then the pro-tier functionalities are automatically unlocked ("it just works").

- suibase-daemon auto-retry on server failures and is sui-aware to avoid equivocation. In other words, sui transactions are sent only once to one server, while "read-only" queries can safely be auto-retried with more than one backend service.

- Efficiently routes each gRPC request depending on being real-time versus static. Immutable data is served faster by traditional backend CDN/caching layers. Real-time/streaming connections are transparently routed and stick to the best RPC server, with automated authentication (as needed).

- suibase-daemon is similar to a client-side load balancer with lookaside:
https://grpc.io/blog/grpc-load-balancing/#lookaside-load-balancing


## Proto Files Location

The gRPC proto files are defined by MystenLabs in the Sui repository:
- Location: `MystenLabs/sui/crates/sui-rpc-api/proto/`
- These define all service interfaces and message types
- Suibase will use these existing definitions, not create new ones

## Sui Repository Management (For Claude Code)

**Recommendation**: Clone locally for better performance and reliability.

**Location**: Outside the Suibase project to avoid confusion
- Suggested: `~/repos/sui-reference/` or `~/reference/sui/`
- NOT in `~/suibase/` (keep project clean)

```bash
# Create reference directory
mkdir -p ~/repos
cd ~/repos

# Clone with shallow history (faster, smaller)
git clone --depth 1 --branch main https://github.com/MystenLabs/sui.git sui-reference

# Or sparse checkout (only what we need)
git clone --filter=blob:none --sparse https://github.com/MystenLabs/sui.git sui-reference
cd sui-reference
git sparse-checkout set crates/sui-node crates/sui-proxy crates/sui-json-rpc crates/sui-rpc-api
```

**Benefits of Local Clone**:
- Faster access (no network latency)
- Can search/grep across files efficiently
- Works offline
- Stable reference (won't change during implementation)

**Best Practices**:
1. Use shallow clone (--depth 1) to save space
2. Reference specific commit in implementation notes
3. Only clone directories needed (sparse checkout)
4. Don't modify the reference repo

## Core Architecture Components

### 1. Protocol Multiplexing (gRPC + JSON-RPC)

Single port (44340) serves both protocols using Content-Type detection:
- `application/grpc` → Tonic handler
- `application/json` → Axum handler

Tower service wrapper, h2c for HTTP/2, matches Mysten Labs approach.

**Reference Implementation**:
- GitHub: https://github.com/MystenLabs/sui (see `crates/sui-node/src/lib.rs`)
- Blog explaining Tower technique: https://academy.fpblock.com/blog/axum-hyper-tonic-tower-part4/
- Pattern: Tower's `Service` trait to route based on Content-Type header


### 2. Authentication System (JWT)

**Challenge-Response Flow**:
1. Client sends Sui address
2. Daemon generates nonce, stores in memory (60s TTL)
3. Client signs nonce with private key
4. Daemon verifies signature, issues JWT

**Challenge Storage**:
- In-memory HashMap: `challenge → {sui_address, timestamp}`
- TTL: 60 seconds
- Cleanup: Every 30 seconds remove expired entries

**JWT Management**:
- Signing: Ed25519 key pair generated on startup (memory only)
- Token TTL: 5 minutes
- Renewal: Request new token at 4 minutes (1 minute overlap)
- Multi-token: Keep last 2 tokens valid (sliding window)

**JWT Claims**:
```json
{
  "sub": "0x123...",     // Sui address
  "jti": "uuid-v4",      // Token ID
  "tier": "pro",         // Service tier
  "rate_limit": 10000,   // RPM limit (free tier only)
  "iat": 1234567890,     // Issued at
  "exp": 1234568190,     // Expires (5 min)
  "iss": "suibase"       // Issuer
}
```

**Rate Limiting Architecture**:
- **Free Tier**: Rate limit enforced locally per daemon (in JWT claims)
- **Pro Tier**: No rate limit in JWT, usage tracked separately (see Payment System)

### 3. Payment System (SUIF Token)

**Applies to pro-tier only** (free-tier always available)

**Pricing**:
- 1 SUIF = 1 request
- Streaming: 1 SUIF/minute + 100 SUIF/MB
- Batch: 1 SUIF/request + 100 SUIF/MB

**Technical Architecture**:

**Hybrid Enforcement Model** (similar to existing rate limiting):

```
Two-Layer Protection:

1. Client-Side (suibase-daemon proxy):
   - Immediate rate limiting (free tier: JWT claims)
   - Usage tracking (pro tier: local counter)
   - Optimistic enforcement for good UX
   - Blocks obvious violations instantly
   - Caches balance checks (30s)

2. Server-Side (backend enforcement):
   - Validates all requests (JWT signature)
   - Asynchronous usage tracking (no blocking)
   - Handles bypass attempts
   - Eventually consistent across servers
   - Final enforcement point

Benefits of Hybrid Approach:
- Fast response for honest users (client-side)
- Security against malicious users (server-side)
- Reduces server load (client filters most violations)
- Better UX (immediate feedback)
- Proven pattern (already used for rate limiting)

Rate Limiting Implementation:
Free Tier:
  - Client: Token bucket in daemon (100 req/min)
  - Server: Validates rate limit in JWT claims

Pro Tier:
  - Client: Usage counter + cached balance check
  - Server: Real usage tracking + on-chain verification
  - Settlement: Batch updates every 60s
```

**Why This Works**:
- Most users are honest → client-side handles 99% of cases
- Malicious users are caught → server-side enforcement
- Efficient → reduces unnecessary server-side checks
- Familiar pattern → extends existing rate limiting approach

**Server-Side Performance Optimization**:
```
Asynchronous Usage Tracking (No Blocking):

1. Request Processing:
   - Validate JWT signature (fast, local)
   - Check local usage cache (in-memory)
   - Process request immediately (no coordination)
   - Async increment usage counter (fire-and-forget)

2. Usage Aggregation:
   - Each server tracks locally (no sync needed)
   - Periodic batch report to central store (60s)
   - Eventually consistent model
   - Tolerate slight overages for performance

3. Geographic Distribution:
   - Each region has local usage cache
   - Regional aggregation every 10s
   - Global reconciliation every 60s
   - Accept temporary discrepancies

Example Flow:
- User has 1000 SUIF balance
- Makes 100 req/s across 3 servers
- Each server allows up to 1100 locally (10% buffer)
- Reconciliation catches overages within 60s
- Slight overage absorbed, user warned/throttled

Trade-offs:
- Performance: <1ms overhead per request
- Accuracy: ~10% temporary overage possible
- Settlement: Exact reconciliation every 60s
- Acceptable for most use cases
```

**Throttle State Broadcasting**:
```
Efficient Throttle Checking (No Performance Kill):

1. Local Bloom Filter (Fast Path):
   - Each server maintains bloom filter of throttled users
   - Check: O(1), ~10 nanoseconds
   - False positives: ~1% (acceptable, just check cache)
   - Updated every 10s from regional cache
   - Size: ~1MB for 1M throttled users

2. Regional Cache (Second Check):
   - LRU cache of throttled users (last 10K)
   - Only checked on bloom filter hit
   - Check: O(1), ~100 nanoseconds
   - Updated from global state every 10s

3. Throttle State Structure:
   {
     address: "0x123...",
     state: "warned" | "throttled" | "suspended",
     expires_at: timestamp,
     overage_amount: 1234
   }

4. Broadcasting Strategy:
   - NOT immediate broadcast (would kill performance)
   - Batch updates every 10s (regional)
   - Critical suspensions: Priority queue (1s)
   - Use pub/sub, not polling

Performance Impact:
- Normal request (99%): 10ns bloom filter check
- Throttled user: 10ns + 100ns cache check
- Network overhead: Minimal (batch updates)
- Memory: ~2MB per server total

Why This Works:
- Bloom filter catches 99% with near-zero cost
- Regional caching reduces network calls
- Batch updates prevent broadcast storms
- Delayed propagation (10s) acceptable for throttling
```

**Token Distribution**:
- Daily via Autocoins feature
- Airdrops for Walrus delegators
- Contributor rewards
- On-chain trading (future)


### 4. Tiered Service Architecture

**Free Tier**:
- Access to Static and Cached methods only
- Limited Real-Time methods (with slight delay)
- Rate limit: 100 requests/min, 3 requests/sec
- May use different port for traffic shaping
- CDN-accelerated responses
- No authentication required
- 2 epochs of data retention.

**Pro Tier**:
- Full access to all methods
- Priority processing
- Rate limit: ~5000 requests/minute
- Full streaming support
- JWT authentication required
- Pay-per-request billing
- full data retention (since genesis)

### 5. Load Balancing & Failover

**Architecture**: Smart proxy with embedded load balancing (not true lookaside)

**Service Discovery via suibase.yaml**:
```yaml
links:
  - alias: "grpc-local"
    grpc: "0.0.0.0:44372"
    priority: 10
  - alias: "grpc-remote"
    grpc: "grpc.suiftly.io:443"
    priority: 20
```

**Server Selection** (reuses existing logic):
- Tier 1: Best latency + servers within threshold
- Tier 2: Healthy but slower
- Tier 3: Degraded (last resort)
- Weak randomization within Tier 1
- Sticky sessions for streams
- X-SBSD-SERVER-IDX header for testing

**Connection Management**:
- HTTP/2 multiplexing: multiple streams per connection
- Connection pool: 1-3 persistent connections per backend
- Health checks: gRPC health protocol
- Circuit breaker: Open after 5 consecutive failures

**CDN Optimization**:
- Static/Cached methods served via CDN for free tier
- Pro tier bypasses CDN for lower latency

**Caching Architecture**:

See [GRPC_CACHE_FEATURE.md](./GRPC_CACHE_FEATURE.md) for implementation details.

**Cache Pool Mapping**:
```
Method Category → Cache Pool → CDN Strategy
Static         → Static Pool → CDN infinite TTL
Cached         → TTL Pool    → CDN with TTL
Real-Time      → No Cache    → Direct to origin
[Dedup]        → Micro Pool  → 10ms deduplication
```

**Compression Strategy**:
- Default: zstd from all Suibase servers
- Transcoding: Stream-based to avoid latency spikes
- Performance: Start sending before complete transcoding
- Fallback: Cache both formats for hot data

## gRPC Methods Classification

### Method Categories

Methods are classified into three categories based on their caching characteristics:

1. **Static**: Immutable data, infinite cache TTL, perfect for CDN
2. **Cached**: Short-lived cache (5-60 seconds), suitable for CDN with TTL
3. **Real-Time**: No caching, requires direct server processing

### Complete Method Inventory

#### LedgerService (Read-Only)

GetServiceInfo                static      free     Service metadata and capabilities
GetObject                     static      free     Retrieve object by ID at specific version
BatchGetObjects               static      free     Retrieve multiple objects
GetTransaction                static      free     Get finalized transaction details
BatchGetTransactions          static      free     Get multiple transactions
GetCheckpoint                 static      free     Get checkpoint by sequence
GetCheckpointAtEpoch          static      free     Get checkpoint at epoch boundary
GetEpoch                      CDN 30s     free     Get epoch information
GetProtocolConfig             CDN 300s    free     Get protocol configuration
GetChainIdentifier            static      free     Get chain ID

#### LiveDataService (Dynamic Queries)

GetLatestCheckpoint           RT          free     Current highest checkpoint
GetReferenceGasPrice          CDN 5s      free     Current gas price
GetSystemState                CDN 10s     free     System state summary
GetValidators                 CDN 60s     free     Current validator set
GetCommittee                  CDN 300s    free     Current committee info
ListDynamicFields             RT          pro      Dynamic object fields
ListOwnedObjects              RT          pro      Objects owned by address
GetCoinInfo                   static      free     Coin metadata
GetBalance                    RT          pro      Account balance
ListBalances                  RT          pro      All balances for address
GetTotalSupply                CDN 60s     free     Total supply of coin
SimulateTransaction           RT          pro      Dry-run transaction
GetDynamicFieldObject         RT          pro      Get dynamic field value

#### TransactionExecutionService (Write Operations)

ExecuteTransaction            RT          pro      Submit signed transaction
ExecuteTransactionBlock       RT          pro      Submit transaction block
BatchExecuteTransactions      RT          pro      Submit multiple transactions

#### SubscriptionService (Streaming)

SubscribeCheckpoints          RT          pro      Stream new checkpoints
SubscribeTransactions         RT          pro      Stream matching transactions
SubscribeEvents               RT          pro      Stream matching events
SubscribeSystemEvents         RT          pro      Stream system events

#### MovePackageService (Move Queries)

GetPackage                    static      free     Get package metadata
GetModule                     static      free     Get module bytecode
GetDatatype                   static      free     Get struct/enum definition
GetFunction                   static      free     Get function signature
ListPackageVersions           static      free     Package upgrade history
GetNormalizedModule           static      free     Get normalized Move module
GetNormalizedFunction         static      free     Get normalized function
GetNormalizedStruct           static      free     Get normalized struct
ResolveNameService            CDN 300s    free     Resolve SuiNS names

#### SignatureVerificationService

VerifySignature               RT          free     Verify Ed25519 signature
VerifyMultisig                RT          pro      Verify multisig
VerifyZkLogin                 RT          pro      Verify zkLogin proof

#### GovernanceService

GetStakingPoolInfo            CDN 60s     free     Staking pool details
GetDelegatedStakes            RT          pro      Delegation information
GetValidatorAPY               CDN 300s    free     Validator APY metrics
GetStakingRewards             RT          pro      Calculate staking rewards

#### EventQueryService

QueryEvents                   RT          pro      Query historical events
GetEventsByTransaction        static      free     Events for transaction
GetEventsByModule             RT          pro      Events by Move module
GetEventsByObject             RT          pro      Events for object

#### IndexerService (Advanced Queries)

QueryTransactionBlocks        RT          pro      Complex transaction queries
QueryObjects                  RT          pro      Complex object queries
GetTransactionKind            static      free     Analyze transaction type
GetObjectsOwnedByAddress      RT          pro      All objects for address
GetObjectsOwnedByObject       RT          pro      Child objects

## CDN Strategy

### Static Methods (Infinite Cache)
- Cache at edge locations indefinitely
- Use content-addressable storage
- Purge only on protocol upgrades

### Cached Methods (TTL-based)
- Cache-Control headers with max-age
- Vary by authentication token for personalized data
- Use stale-while-revalidate for better UX

### Real-Time Methods
- Bypass CDN entirely for pro tier
- Free tier through separate rate-limited endpoint
- WebSocket upgrade for streaming

## Technical Specifications

### Performance Targets
- Static: <10ms (CDN edge)
- Cached: <50ms (cache hit)
- Real-time: <100ms (direct)
- Streaming: <200ms (initial)

### Error Handling & Resilience
- Request timeout: 30s default (configurable per method)
- Stream timeout: 5 minutes
- Retry policy: Max 3 attempts (100ms, 500ms, 2s backoff)
- Circuit breaker: 5 consecutive failures to open
- Backpressure: HTTP/2 flow control for streams

### Security
- TLS 1.3 for external connections
- h2c (cleartext) for local port
- Rate limiting per tier
- DDoS protection via CDN

### Monitoring
- OpenTelemetry integration
- Per-method latency/errors
- Usage tracking per address

## Configuration Schema

**suibase.yaml additions**:
```yaml
grpc:
  enabled: true
  port: 44372
  h2c_enabled: true
  max_concurrent_streams: 100

grpc_auth:
  enabled: true
  challenge_ttl_seconds: 60
  token_ttl_seconds: 300
  max_tokens_per_user: 2

grpc_cache:  # See GRPC_CACHE_FEATURE.md
  enabled: true
  static_memory_mb: 150
  static_disk_gb: 1
  ttl_memory_mb: 100
  micro_memory_mb: 10
```

## State Management

**Single Instance**: suibase-daemon runs as singleton (no coordination needed)
- All state in-memory or local disk
- No distributed state management
- Restarts regenerate ephemeral state (JWT keys, challenges)

## Future Enhancements

### Phase 2 Considerations
- GraphQL gateway over gRPC
- Custom query language
- Advanced caching strategies
- Multi-region deployment

### Phase 3 Possibilities
- WebAssembly user-defined functions
- Custom indexing services
- Analytics and monitoring dashboards
- Enterprise SLAs with guaranteed latency