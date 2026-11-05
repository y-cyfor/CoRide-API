use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Parse an IP access-control rule.
///
/// Accepted forms:
/// - plain address: `203.0.113.7`, `2001:db8::1`
/// - IPv4 CIDR: `203.0.113.0/24`
/// - IPv6 CIDR: `2001:db8::/32`
///
/// Returns `(network, prefix_len)` for CIDR rules and `(addr, full_len)` for plain
/// addresses.
pub fn parse_rule(rule: &str) -> Option<(IpAddr, u8)> {
    let rule = rule.trim();
    if rule.is_empty() || rule.len() > 128 {
        return None;
    }

    if let Some((addr_part, prefix_part)) = rule.split_once('/') {
        let addr: IpAddr = addr_part.parse().ok()?;
        let prefix: u8 = prefix_part.parse().ok()?;
        let max = match addr {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        if prefix > max {
            return None;
        }
        // Normalise the network address so that stored rules compare consistently.
        Some((mask_address(addr, prefix), prefix))
    } else {
        let addr: IpAddr = rule.parse().ok()?;
        let full = match addr {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        Some((addr, full))
    }
}

/// Validate a rule without normalising it (used by the admin API).
pub fn is_valid_rule(rule: &str) -> bool {
    parse_rule(rule).is_some()
}

/// Canonical string for a rule (network address + prefix, plain address otherwise).
pub fn normalise_rule(rule: &str) -> Option<String> {
    let (addr, prefix) = parse_rule(rule)?;
    let full = match addr {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    };
    if prefix == full {
        Some(addr.to_string())
    } else {
        Some(format!("{addr}/{prefix}"))
    }
}

/// Return true when `ip` matches `rule` (exact address or CIDR block).
pub fn matches(ip: &str, rule: &str) -> bool {
    let Ok(ip) = ip.trim().parse::<IpAddr>() else {
        return false;
    };
    // Unparseable rule => fall back to plain string comparison so that legacy rows
    // entered before validation still behave predictably.
    let Some((network, prefix)) = parse_rule(rule) else {
        return ip.to_string() == rule.trim();
    };

    match (ip, network) {
        // Masking both sides keeps exact addresses (full prefix) and CIDR blocks on the
        // same code path; the earlier `prefix >= 32` shortcut matched every IPv4.
        (IpAddr::V4(ip), IpAddr::V4(net)) => v4_masked(ip, prefix) == net,
        (IpAddr::V6(ip), IpAddr::V6(net)) => v6_masked(ip, prefix) == net,
        _ => false,
    }
}

/// Return true when any rule in `rules` matches `ip`.
pub fn matches_any(ip: &str, rules: &[String]) -> bool {
    rules.iter().any(|rule| matches(ip, rule))
}

fn mask_address(addr: IpAddr, prefix: u8) -> IpAddr {
    match addr {
        IpAddr::V4(v4) => IpAddr::V4(v4_masked(v4, prefix)),
        IpAddr::V6(v6) => IpAddr::V6(v6_masked(v6, prefix)),
    }
}

fn v4_masked(addr: Ipv4Addr, prefix: u8) -> Ipv4Addr {
    if prefix == 0 {
        return Ipv4Addr::UNSPECIFIED;
    }
    let bits = u32::from(addr);
    let mask = if prefix >= 32 {
        u32::MAX
    } else {
        u32::MAX << (32 - prefix)
    };
    Ipv4Addr::from(bits & mask)
}

fn v6_masked(addr: Ipv6Addr, prefix: u8) -> Ipv6Addr {
    if prefix == 0 {
        return Ipv6Addr::UNSPECIFIED;
    }
    let bits = u128::from(addr);
    let mask = if prefix >= 128 {
        u128::MAX
    } else {
        u128::MAX << (128 - prefix)
    };
    Ipv6Addr::from(bits & mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_ipv4_matches() {
        assert!(matches("203.0.113.7", "203.0.113.7"));
        assert!(!matches("203.0.113.8", "203.0.113.7"));
    }

    #[test]
    fn ipv4_cidr_matches() {
        assert!(matches("192.168.1.5", "192.168.1.0/24"));
        assert!(matches("192.168.1.255", "192.168.1.0/24"));
        assert!(!matches("192.168.2.1", "192.168.1.0/24"));
        assert!(matches("10.1.2.3", "10.0.0.0/8"));
        assert!(matches("8.8.8.8", "0.0.0.0/0"));
    }

    #[test]
    fn ipv6_cidr_matches() {
        assert!(matches("2001:db8::1", "2001:db8::/32"));
        assert!(matches("2001:db8:ffff::9", "2001:db8::/32"));
        assert!(!matches("2001:db9::1", "2001:db8::/32"));
    }

    #[test]
    fn ipv6_alias_forms_match() {
        // Same address, different textual forms.
        assert!(matches("::1", "0:0:0:0:0:0:0:1"));
        assert!(matches("0:0:0:0:0:0:0:1", "::1"));
    }

    #[test]
    fn family_mismatch_never_matches() {
        assert!(!matches("192.168.1.1", "::/0"));
        assert!(!matches("::1", "0.0.0.0/0"));
    }

    #[test]
    fn invalid_rules_are_rejected() {
        assert!(!is_valid_rule("hello"));
        assert!(!is_valid_rule("192.168.1.0/33"));
        assert!(!is_valid_rule("2001:db8::/129"));
        assert!(!is_valid_rule(""));
        assert!(is_valid_rule("192.168.1.0/24"));
        assert!(is_valid_rule("::1"));
    }

    #[test]
    fn normalisation_canonicalises_rules() {
        assert_eq!(normalise_rule("192.168.1.5/24").unwrap(), "192.168.1.0/24");
        assert_eq!(normalise_rule("::1").unwrap(), "::1");
        assert_eq!(normalise_rule("0:0:0:0:0:0:0:1").unwrap(), "::1");
        assert_eq!(normalise_rule("192.168.1.1").unwrap(), "192.168.1.1");
    }

    #[test]
    fn matches_any_checks_every_rule() {
        let rules = vec!["10.0.0.0/8".to_string(), "203.0.113.9".to_string()];
        assert!(matches_any("10.9.9.9", &rules));
        assert!(matches_any("203.0.113.9", &rules));
        assert!(!matches_any("203.0.113.10", &rules));
    }
}
