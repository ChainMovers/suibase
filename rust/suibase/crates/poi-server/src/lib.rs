//! POI server placeholder implementation.
//!
//! Backend server code is not public.
//!
pub fn poi_server_main() {
    println!("POI server");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        poi_server_main();
        assert!(true);
    }
}
