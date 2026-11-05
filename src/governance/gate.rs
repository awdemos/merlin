//! Eligibility gate: pure function from (app policy, sensitivity, route
//! scope) to the eligible target set. An empty set is a denial with
//! self-explaining conflicts — never a fall-through to a cheaper target.

use super::apps::{AppPolicy, LocalityPolicy};
use std::net::IpAddr;
use std::str::FromStr;

/// Resolved sensitivity of a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sensitivity {
    Clean,
    Sensitive,
    /// Sensitivity could not be determined (detector unavailable); the
    /// gate fails closed and applies the strictest policy.
    Undetermined,
}

/// One candidate target: its name, whether it runs locally, and whether
/// it declares pricing (needed for spend-capped apps to charge accurately).
#[derive(Debug, Clone)]
pub struct TargetScope {
    pub name: String,
    pub local: bool,
    pub priced: bool,
}

/// Why a request was denied, in terms the caller can act on. Hints never
/// suggest relaxing a security requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Denial {
    pub code: &'static str,
    pub conflicts: Vec<String>,
}

/// Compute the eligible target set for a request.
///
/// Restrictions compose:
/// - `allowed_targets` allowlist (if set) intersects the scope;
/// - `locality = LocalOnly` keeps only local targets;
/// - `Sensitive` + `sensitive_local_only` keeps only local targets
///   (raise-only; never applied when the request is clean);
/// - `Undetermined` keeps only local targets regardless of policy
///   (fail-closed);
/// - a daily spend cap keeps only priced targets, so the cap can actually
///   bind (fail-closed; unpriced targets would bypass it).
pub fn evaluate(
    policy: &AppPolicy,
    sensitivity: Sensitivity,
    scope: &[TargetScope],
) -> Result<Vec<String>, Denial> {
    let strict_local = sensitivity == Sensitivity::Undetermined;
    let restrict_local = policy.locality == LocalityPolicy::LocalOnly
        || (sensitivity == Sensitivity::Sensitive && policy.sensitive_local_only)
        || strict_local;
    let cap_active = policy.max_spend_usd_per_day.is_some();

    let eligible: Vec<String> = scope
        .iter()
        .filter(|t| !restrict_local || t.local)
        .filter(|t| {
            policy
                .allowed_targets
                .as_ref()
                .map(|allowed| allowed.iter().any(|a| a == &t.name))
                .unwrap_or(true)
        })
        .filter(|t| !cap_active || t.priced)
        .map(|t| t.name.clone())
        .collect();

    if !eligible.is_empty() {
        return Ok(eligible);
    }

    let mut conflicts = Vec::new();
    let has_local = scope.iter().any(|t| t.local);
    if restrict_local {
        if strict_local {
            conflicts.push(
                "sensitivity undetermined: strict policy requires local targets, but none available"
                    .to_string(),
            );
        } else if has_local {
            conflicts.push("no local targets eligible under the active policy".to_string());
        } else {
            conflicts.push(
                "policy requires local targets, but no local targets are available".to_string(),
            );
        }
    }
    if let Some(allowed) = &policy.allowed_targets {
        if scope.iter().all(|t| !allowed.contains(&t.name)) {
            conflicts.push("allowed_targets excludes every target in this route".to_string());
        }
    }
    if cap_active && !scope.iter().any(|t| t.priced) {
        conflicts.push("spend cap requires priced targets, but none are available".to_string());
    }
    if conflicts.is_empty() {
        conflicts.push("no targets in scope".to_string());
    }

    Err(Denial {
        code: "policy_denied",
        conflicts,
    })
}

/// True when `base_url`'s host is localhost, loopback, or a private-range
/// IP — i.e. traffic to this client stays on local infrastructure. The
/// check is fail-closed: anything that does not parse as a concrete local
/// IP (including hostname strings that merely start with local digits,
/// like `127.0.0.1.evil.com`) is classified non-local.
pub fn host_is_local(base_url: &str) -> bool {
    // Lowercasing first makes the scheme trim case-insensitive; hostnames
    // and IPv6 hex are case-insensitive, so the host is unaffected.
    let lower = base_url.to_ascii_lowercase();
    let rest = lower
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    // An IPv6 literal is bracketed; its colons are part of the address,
    // so the host must be extracted before splitting off port/path.
    let host = if let Some(inner) = rest.strip_prefix('[') {
        inner.split(']').next().unwrap_or("")
    } else {
        rest.split(['/', ':']).next().unwrap_or("")
    };
    if host == "localhost" {
        return true;
    }
    // Tolerate the FQDN trailing dot on IP literals ("10.0.0.1.").
    let host = host.strip_suffix('.').unwrap_or(host);
    match IpAddr::from_str(host) {
        Ok(IpAddr::V4(ip)) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        Ok(IpAddr::V6(ip)) => ip.is_loopback(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(names: &[(&str, bool)]) -> Vec<TargetScope> {
        names
            .iter()
            .map(|(n, local)| TargetScope {
                name: n.to_string(),
                local: *local,
                priced: true,
            })
            .collect()
    }

    #[test]
    fn default_policy_allows_everything() {
        let policy = AppPolicy::default();
        let result = evaluate(
            &policy,
            Sensitivity::Clean,
            &scope(&[("a", false), ("b", true)]),
        );
        assert_eq!(result.unwrap(), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn local_only_keeps_local_targets() {
        let policy = AppPolicy {
            locality: LocalityPolicy::LocalOnly,
            ..Default::default()
        };
        let result = evaluate(
            &policy,
            Sensitivity::Clean,
            &scope(&[("cloud", false), ("ollama", true)]),
        );
        assert_eq!(result.unwrap(), vec!["ollama".to_string()]);
    }

    #[test]
    fn local_only_with_no_local_targets_denies_with_conflict() {
        let policy = AppPolicy {
            locality: LocalityPolicy::LocalOnly,
            ..Default::default()
        };
        let denial =
            evaluate(&policy, Sensitivity::Clean, &scope(&[("cloud", false)])).unwrap_err();
        assert_eq!(denial.code, "policy_denied");
        assert!(denial
            .conflicts
            .iter()
            .any(|c| c.contains("no local targets")));
    }

    #[test]
    fn allowed_targets_allowlist_filters() {
        let policy = AppPolicy {
            allowed_targets: Some(vec!["b".to_string()]),
            ..Default::default()
        };
        let result = evaluate(
            &policy,
            Sensitivity::Clean,
            &scope(&[("a", false), ("b", true)]),
        );
        assert_eq!(result.unwrap(), vec!["b".to_string()]);
    }

    #[test]
    fn sensitive_with_raise_only_restricts_to_local() {
        let policy = AppPolicy {
            sensitive_local_only: true,
            ..Default::default()
        };
        let sensitive = evaluate(
            &policy,
            Sensitivity::Sensitive,
            &scope(&[("cloud", false), ("ollama", true)]),
        );
        assert_eq!(sensitive.unwrap(), vec!["ollama".to_string()]);
        let clean = evaluate(
            &policy,
            Sensitivity::Clean,
            &scope(&[("cloud", false), ("ollama", true)]),
        );
        assert_eq!(clean.unwrap().len(), 2);
    }

    #[test]
    fn sensitive_without_raise_only_stays_on_default_policy() {
        let policy = AppPolicy::default();
        let result = evaluate(
            &policy,
            Sensitivity::Sensitive,
            &scope(&[("cloud", false), ("ollama", true)]),
        );
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn undetermined_is_strict_even_without_raise_only() {
        let policy = AppPolicy::default();
        let result = evaluate(
            &policy,
            Sensitivity::Undetermined,
            &scope(&[("cloud", false), ("ollama", true)]),
        );
        assert_eq!(result.unwrap(), vec!["ollama".to_string()]);
    }

    #[test]
    fn undetermined_with_no_local_targets_denies() {
        let policy = AppPolicy::default();
        let denial = evaluate(
            &policy,
            Sensitivity::Undetermined,
            &scope(&[("cloud", false)]),
        )
        .unwrap_err();
        assert!(denial.conflicts.iter().any(|c| c.contains("undetermined")));
    }

    #[test]
    fn allowlist_and_locality_compose() {
        let policy = AppPolicy {
            allowed_targets: Some(vec!["cloud".to_string(), "local-b".to_string()]),
            locality: LocalityPolicy::LocalOnly,
            ..Default::default()
        };
        let denial = evaluate(
            &policy,
            Sensitivity::Clean,
            &scope(&[("cloud", false), ("ollama", true)]),
        );
        assert!(denial.is_err());
        let both = evaluate(
            &policy,
            Sensitivity::Clean,
            &scope(&[("cloud", false), ("local-b", true)]),
        );
        assert_eq!(both.unwrap(), vec!["local-b".to_string()]);
    }

    #[test]
    fn capped_policy_excludes_unpriced_targets() {
        let policy = AppPolicy {
            max_spend_usd_per_day: Some(1.0),
            ..Default::default()
        };
        let scope = vec![
            TargetScope {
                name: "priced".to_string(),
                local: true,
                priced: true,
            },
            TargetScope {
                name: "unpriced".to_string(),
                local: true,
                priced: false,
            },
        ];
        let result = evaluate(&policy, Sensitivity::Clean, &scope);
        assert_eq!(result.unwrap(), vec!["priced".to_string()]);
    }

    #[test]
    fn capped_policy_with_only_unpriced_targets_denies() {
        let policy = AppPolicy {
            max_spend_usd_per_day: Some(1.0),
            ..Default::default()
        };
        let scope = vec![TargetScope {
            name: "unpriced".to_string(),
            local: true,
            priced: false,
        }];
        let denial = evaluate(&policy, Sensitivity::Clean, &scope).unwrap_err();
        assert_eq!(denial.code, "policy_denied");
        assert!(denial.conflicts.iter().any(|c| c.contains("priced")));
    }

    #[test]
    fn uncapped_policy_keeps_unpriced_targets() {
        let policy = AppPolicy::default();
        let scope = vec![TargetScope {
            name: "unpriced".to_string(),
            local: false,
            priced: false,
        }];
        let result = evaluate(&policy, Sensitivity::Clean, &scope);
        assert_eq!(result.unwrap(), vec!["unpriced".to_string()]);
    }

    #[test]
    fn locality_derivation() {
        assert!(host_is_local("http://127.0.0.1:11434"));
        assert!(host_is_local("http://localhost:8000/v1"));
        assert!(host_is_local("http://10.0.0.5:9000"));
        assert!(host_is_local("http://192.168.1.20"));
        assert!(host_is_local("http://172.16.0.5"));
        assert!(host_is_local("http://172.31.255.1"));
        assert!(host_is_local("http://[::1]:8080"));
        assert!(!host_is_local("https://api.openai.com/v1"));
        assert!(!host_is_local("https://172.32.0.1"));
        assert!(!host_is_local("http://172.15.0.1"));
        assert!(!host_is_local("https://example.com"));
    }

    #[test]
    fn locality_parser_failure_modes_are_fail_closed() {
        assert!(!host_is_local(""));
        assert!(!host_is_local("http://"));
        // Scheme match is case-insensitive.
        assert!(host_is_local("HTTP://10.0.0.1"));
        // "localhost." does not exactly match localhost and is not an IP.
        assert!(!host_is_local("http://localhost."));
        // Hostnames that merely start with local-looking digits.
        assert!(!host_is_local("http://127.0.0.1.evil.com"));
        assert!(!host_is_local("http://10.evil.com"));
        assert!(!host_is_local("http://192.168.1.1.attacker.io"));
        // Bare IPv6 without brackets does not parse; global v6 is not local.
        assert!(!host_is_local("::1"));
        assert!(!host_is_local("http://[2001:db8::1]:9000"));
        assert!(host_is_local("HTTP://10.0.0.1/v1"));
        assert!(host_is_local("localhost:11434"));
        assert!(host_is_local("http://[::1]:8080/v1"));
        assert!(host_is_local("http://169.254.10.5/"));
        assert!(!host_is_local("https://172.32.0.1"));
        assert!(!host_is_local("http://172.15.0.1"));
    }
}
