//! Property test: for any policy, sensitivity, and route scope, the
//! eligible set never contains a target the policy forbids. This is the
//! fail-open canary — RouteForge's Hypothesis harness caught two such
//! defects; ours proves the invariant at the seam.

use merlin::governance::apps::{AppPolicy, LocalityPolicy};
use merlin::governance::gate::{evaluate, host_is_local, Sensitivity, TargetScope};
use proptest::prelude::*;

fn scope_strategy() -> impl Strategy<Value = Vec<TargetScope>> {
    proptest::collection::vec(("[a-z]{1,8}", any::<bool>()), 0..6).prop_map(|items| {
        items
            .into_iter()
            .map(|(name, local)| TargetScope { name, local })
            .collect()
    })
}

fn policy_strategy() -> impl Strategy<Value = AppPolicy> {
    (
        proptest::option::of(proptest::collection::vec("[a-z]{1,8}", 0..4)),
        any::<bool>(),
        proptest::option::of(0u32..1000),
        proptest::option::of(0.0f64..100.0),
        any::<bool>(),
    )
        .prop_map(
            |(allowed_targets, local_only, max_rpm, max_spend, sensitive_local_only)| AppPolicy {
                allowed_targets,
                locality: if local_only {
                    LocalityPolicy::LocalOnly
                } else {
                    LocalityPolicy::Any
                },
                max_rpm,
                max_spend_usd_per_day: max_spend,
                sensitive_local_only,
                ..Default::default()
            },
        )
}

proptest! {
    #[test]
    fn eligible_set_never_contains_forbidden_target(
        policy in policy_strategy(),
        sensitive in any::<bool>(),
        undetermined in any::<bool>(),
        scope in scope_strategy(),
    ) {
        let sensitivity = if undetermined {
            Sensitivity::Undetermined
        } else if sensitive {
            Sensitivity::Sensitive
        } else {
            Sensitivity::Clean
        };

        if let Ok(eligible) = evaluate(&policy, sensitivity, &scope) {
            let restricted = policy.locality == LocalityPolicy::LocalOnly
                || (sensitivity == Sensitivity::Sensitive && policy.sensitive_local_only)
                || sensitivity == Sensitivity::Undetermined;
            for name in &eligible {
                let target = scope.iter().find(|t| &t.name == name).expect("name from scope");
                prop_assert!(!restricted || target.local,
                    "non-local target {name} in eligible set under local restriction");
                if let Some(allowed) = &policy.allowed_targets {
                    prop_assert!(allowed.contains(name),
                        "target {name} not in allowed_targets");
                }
            }
        }
    }

    /// Fail-closed canary for the URL locality parser: only the exact
    /// string "localhost" (and IP literals, excluded here by construction)
    /// is local. Any hostname with alphabetic characters beyond that exact
    /// string must be non-local.
    #[test]
    fn only_exact_localhost_hostname_is_local(host in hostname_strategy()) {
        prop_assume!(host != "localhost");
        prop_assert!(!host_is_local(&format!("http://{host}:8080")),
            "hostname {host} classified local without being exactly localhost");
    }
}

fn hostname_strategy() -> impl Strategy<Value = String> {
    (
        proptest::collection::vec("[a-z0-9]{1,10}", 1..4),
        any::<bool>(),
    )
        .prop_map(|(mut parts, trailing_dot)| {
            // Guarantee at least one alphabetic character so the name can
            // never parse as a numeric IP literal (dot-separated digits).
            if parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
                parts[0].push('x');
            }
            let mut host = parts.join(".");
            if trailing_dot {
                host.push('.');
            }
            host
        })
}
