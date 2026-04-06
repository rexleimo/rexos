use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub(crate) fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_forbidden_ipv4(v4),
        IpAddr::V6(v6) => is_forbidden_ipv6(v6),
    }
}

fn is_forbidden_ipv4(ip: Ipv4Addr) -> bool {
    if ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_unspecified()
    {
        return true;
    }

    let octets = ip.octets();
    octets[0] == 100 && (64..=127).contains(&octets[1])
}

fn is_forbidden_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_loopback()
        || is_unique_local_ipv6(ip)
        || is_unicast_link_local_ipv6(ip)
        || ip.is_multicast()
        || ip.is_unspecified()
    {
        return true;
    }

    let first_segment = ip.segments()[0];
    (first_segment & 0xffc0) == 0xfec0
}

fn is_unique_local_ipv6(ip: Ipv6Addr) -> bool {
    let first_segment = ip.segments()[0];
    (first_segment & 0xfe00) == 0xfc00
}

fn is_unicast_link_local_ipv6(ip: Ipv6Addr) -> bool {
    let first_segment = ip.segments()[0];
    (first_segment & 0xffc0) == 0xfe80
}
