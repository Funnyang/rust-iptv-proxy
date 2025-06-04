use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;
use local_ip_address::list_afinet_netifas;
use log::debug;

static INTERFACE_CACHE: OnceLock<HashMap<String, IpAddr>> = OnceLock::new();

pub fn get_interface_ip(if_name: &str) -> anyhow::Result<Ipv4Addr> {
    let cache = INTERFACE_CACHE.get_or_init(|| {
        let mut cache = HashMap::new();
        if let Ok(interfaces) = list_afinet_netifas() {
            for (name, ip) in interfaces {
                debug!("Caching interface {}: {}", name, ip);
                cache.insert(name, ip);
            }
        }
        cache
    });

    let default_ip = Ipv4Addr::new(0, 0, 0, 0);

    if let Some(ip) = cache.get(if_name) {
        if let IpAddr::V4(ipv4) = ip {
            Ok(*ipv4)
        } else {
            Ok(default_ip)
        }
    } else {
        Ok(default_ip)
    }
}