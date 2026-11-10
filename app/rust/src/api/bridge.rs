#[flutter_rust_bridge::frb(init)]
pub fn init_app() {}

pub fn get_local_ip() -> String {
    localpost::local_ip_address::local_ip()
        .ok()
        .map(|ip| ip.to_string())
        .unwrap_or_default()
}
