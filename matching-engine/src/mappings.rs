// this should eventually all be moved to build script. too lazy atm.

fn match_ip_to_id(ip: Ipv4Addr) -> TraderId {
    match ip {
        Ipv4Addr::new(172,16,123,1) => macro_calls::TraderId::Columbia_A,
    }
}