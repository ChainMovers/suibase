// These integration tests assume:
//  - localnet is already installed
//  - 'demo' package is already published to localnet.

use log;
use suibase::Helper;

fn init() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();
}

#[test]
fn test_is_installed() {
    let sbh = Helper::new();
    assert!(sbh.is_installed().unwrap());
}

#[test]
fn test_localnet() {
    let sbh = Helper::new();
    assert!(sbh.is_installed().unwrap());
    sbh.select_workdir("localnet").unwrap();
    assert_eq!(sbh.workdir().unwrap(), "localnet");
}

#[test]
fn test_demo_package() {
    init();
    let sbh = Helper::new();
    assert!(sbh.is_installed().unwrap());
    sbh.select_workdir("localnet").unwrap();
    assert_eq!(sbh.workdir().unwrap(), "localnet");
    let package_id = sbh.package_id("demo");
    if package_id.is_err() {
        log::error!("Error: {:?}", package_id);
    }
    assert!(package_id.is_ok());
    // Verify package_id is an hex string
    let package_id = package_id.unwrap();
    log::info!("package_id: {} length: {}", package_id, package_id.len());
    assert_eq!(package_id.starts_with("0x"), true);
    assert_eq!(package_id.len(), 66);
}
