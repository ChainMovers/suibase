// Legacy test file - tests have been reorganized into:
// - mock_server_api_tests.rs: Tests for mock server API functionality
// - rate_limiting_tests.rs: Tests for rate limiting functionality  
// - proxy_behavior_tests.rs: Tests for proxy server behavior (load balancing, failover, etc.)
//
// This file is kept to ensure backward compatibility but all tests have been moved
// to more appropriately named files for better organization.

// Re-export the tests from their new locations so existing test commands still work
#[path = "mock_server_api_tests.rs"]
mod mock_server_api_tests;

#[path = "rate_limiting_tests.rs"]
mod rate_limiting_tests;

#[path = "proxy_behavior_tests.rs"]
mod proxy_behavior_tests;