//! Verified pure model of the bandit routing core.
//!
//! This module mirrors the pure update/selection math of the routing
//! algorithms in this directory, so the integer/counting invariants can be
//! machine-checked by Verus:
//!
//! * [`BanditCounts::record_hit`] mirrors the `record_reward` count update of
//!   `epsilon_greedy.rs` and `ucb.rs` (`counts.entry(t).or_insert(0) += 1`).
//! * [`BanditCounts::record_hits`] models a routing session and proves the
//!   total pull count equals the number of selection steps.
//! * [`BanditOutcomes::record_outcome`] mirrors the `record_reward` update of
//!   `thompson.rs` (exactly one of successes/failures grows per call).
//! * [`select_ucb`] mirrors the selection loop of `ucb.rs`; the confidence
//!   bonus formula (which divides by the pull count) is only ever evaluated
//!   for arms with a nonzero pull count.
//! * [`select_epsilon_greedy`]/[`argmax`]/[`explore_pick`] mirror the
//!   exploit/explore selection of `epsilon_greedy.rs`; `explore_pick` also
//!   mirrors the hash-based pick of `contextual.rs`.
//!
//! The real implementations store counts in `Mutex<HashMap<..>>` and sample
//! from `rand`; those are outside the scope of verification. All `f64` math
//! is treated as opaque: it lives in `external_body` helpers whose integer
//! preconditions encode exactly the guards the real code uses (e.g. the UCB
//! bonus divides by `n` only when `n > 0`).

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Spec: sum of a sequence of pull counts, as a mathematical integer.
// ---------------------------------------------------------------------------

pub open spec fn sum_range(s: Seq<u64>, lo: int, hi: int) -> int
    decreases hi - lo
{
    if lo >= hi {
        0
    } else {
        s[lo] as int + sum_range(s, lo + 1, hi)
    }
}

proof fn lemma_sum_nonneg(s: Seq<u64>, lo: int, hi: int)
    requires
        lo <= hi,
    ensures
        0 <= sum_range(s, lo, hi),
    decreases hi - lo
{
    if lo < hi {
        lemma_sum_nonneg(s, lo + 1, hi);
    }
}

proof fn lemma_sum_range_split(s: Seq<u64>, lo: int, mid: int, hi: int)
    requires
        lo <= mid <= hi,
    ensures
        sum_range(s, lo, hi) == sum_range(s, lo, mid) + sum_range(s, mid, hi),
    decreases mid - lo
{
    if lo < mid {
        assert(sum_range(s, lo, hi) == s[lo] as int + sum_range(s, lo + 1, hi));
        lemma_sum_range_split(s, lo + 1, mid, hi);
    } else {
        assert(sum_range(s, lo, hi) == sum_range(s, mid, hi));
    }
}

proof fn lemma_sum_range_mono(s: Seq<u64>, lo: int, mid: int, hi: int)
    requires
        lo <= mid <= hi,
    ensures
        sum_range(s, lo, mid) <= sum_range(s, lo, hi),
    decreases mid - lo
{
    lemma_sum_range_split(s, lo, mid, hi);
    lemma_sum_nonneg(s, mid, hi);
}

proof fn lemma_sum_elem_le(s: Seq<u64>, lo: int, i: int, hi: int)
    requires
        lo <= i < hi,
    ensures
        s[i] as int <= sum_range(s, lo, hi),
    decreases hi - lo
{
    if lo < i {
        lemma_sum_elem_le(s, lo + 1, i, hi);
    } else {
        assert(sum_range(s, lo, hi) == s[lo] as int + sum_range(s, lo + 1, hi));
        lemma_sum_nonneg(s, lo + 1, hi);
    }
}

proof fn lemma_sum_update_outside(s: Seq<u64>, i: int, v: u64, lo: int, hi: int)
    requires
        0 <= lo <= hi,
        i < lo,
        hi <= s.len(),
    ensures
        sum_range(s.update(i, v), lo, hi) == sum_range(s, lo, hi),
    decreases hi - lo
{
    if lo < hi {
        assert(s.update(i, v)[lo] == s[lo]);
        lemma_sum_update_outside(s, i, v, lo + 1, hi);
    }
}

proof fn lemma_sum_update_inside(s: Seq<u64>, i: int, v: u64, lo: int, hi: int)
    requires
        0 <= lo <= i < hi,
        hi <= s.len(),
    ensures
        sum_range(s.update(i, v), lo, hi) == sum_range(s, lo, hi) - s[i] as int + v as int,
    decreases i - lo
{
    if lo < i {
        assert(s.update(i, v)[lo] == s[lo]);
        lemma_sum_update_inside(s, i, v, lo + 1, hi);
    } else {
        assert(lo == i);
        assert(s.update(i, v)[lo] == v);
        lemma_sum_update_outside(s, i, v, lo + 1, hi);
    }
}

proof fn lemma_sum_singleton(s: Seq<u64>, i: int)
    requires
        0 <= i,
    ensures
        sum_range(s, i, i + 1) == s[i] as int,
{
    assert(sum_range(s, i, i + 1) == s[i] as int + sum_range(s, i + 1, i + 1));
    assert(sum_range(s, i + 1, i + 1) == 0);
}

proof fn lemma_sum_push_from(s: Seq<u64>, v: u64, lo: int)
    requires
        0 <= lo <= s.len(),
    ensures
        sum_range(s.push(v), lo, s.len() as int + 1) == sum_range(s, lo, s.len() as int) + v as int,
    decreases s.len() - lo
{
    if lo == s.len() {
        assert(s.push(v)[lo] == v);
        assert(sum_range(s.push(v), lo, s.len() as int + 1) == s.push(v)[lo] as int + sum_range(
            s.push(v),
            lo + 1,
            s.len() as int + 1,
        ));
        assert(sum_range(s.push(v), lo + 1, s.len() as int + 1) == 0);
    } else {
        assert(s.push(v)[lo] == s[lo]);
        assert(sum_range(s.push(v), lo, s.len() as int + 1) == s.push(v)[lo] as int + sum_range(
            s.push(v),
            lo + 1,
            s.len() as int + 1,
        ));
        assert(sum_range(s, lo, s.len() as int) == s[lo] as int + sum_range(s, lo + 1, s.len() as int));
        lemma_sum_push_from(s, v, lo + 1);
    }
}

proof fn lemma_sum_push(s: Seq<u64>, v: u64)
    ensures
        sum_range(s.push(v), 0, s.len() as int + 1) == sum_range(s, 0, s.len() as int) + v as int,
{
    lemma_sum_push_from(s, v, 0);
}

// ---------------------------------------------------------------------------
// Model of the per-target pull-count state (epsilon_greedy.rs, ucb.rs).
// ---------------------------------------------------------------------------

pub struct BanditCounts {
    pub names: Vec<String>,
    pub counts: Vec<u64>,
}

impl BanditCounts {
    pub open spec fn wf(&self) -> bool {
        self.names@.len() == self.counts@.len()
    }

    /// Total number of recorded selections, as a mathematical integer.
    pub open spec fn total_spec(&self) -> int {
        sum_range(self.counts@, 0, self.counts@.len() as int)
    }

    pub fn new() -> (s: Self)
        ensures
            s.wf(),
            s.total_spec() == 0,
    {
        BanditCounts { names: Vec::new(), counts: Vec::new() }
    }

    /// Total pull count. Mirrors `counts.values().sum::<u64>()` in ucb.rs.
    pub fn total(&self) -> (t: u64)
        requires
            self.wf(),
            self.total_spec() <= u64::MAX as int,
        ensures
            t as int == self.total_spec(),
    {
        proof {
            lemma_sum_nonneg(self.counts@, 0, self.counts@.len() as int);
        }
        assert(sum_range(self.counts@, 0, self.counts@.len() as int) == self.total_spec());
        assert(sum_range(self.counts@, 0, self.counts@.len() as int) <= u64::MAX as int);
        let ghost counts_seq: Seq<u64> = self.counts@;
        assert(counts_seq == self.counts@);
        let mut t: u64 = 0;
        let mut i: usize = 0;
        while i < self.counts.len()
            invariant
                i <= self.counts.len(),
                self.counts@ == counts_seq,
                t as int == sum_range(counts_seq, 0, i as int),
                sum_range(counts_seq, 0, i as int) <= sum_range(counts_seq, 0, counts_seq.len() as int),
                sum_range(counts_seq, 0, counts_seq.len() as int) <= u64::MAX as int,
            decreases self.counts.len() - i
        {
            proof {
                lemma_sum_range_mono(counts_seq, 0, (i + 1) as int, counts_seq.len() as int);
                lemma_sum_elem_le(counts_seq, i as int, i as int, counts_seq.len() as int);
                lemma_sum_range_split(counts_seq, 0, i as int, counts_seq.len() as int);
                lemma_sum_range_split(counts_seq, 0, i as int, (i + 1) as int);
                lemma_sum_singleton(counts_seq, i as int);
            }
            assert(counts_seq[i as int] as int == self.counts[i as int] as int);
            assert(sum_range(counts_seq, 0, (i + 1) as int) == sum_range(counts_seq, 0, i as int)
                + counts_seq[i as int] as int);
            assert(sum_range(counts_seq, 0, i as int) + sum_range(counts_seq, i as int, counts_seq.len() as int) == sum_range(counts_seq, 0, counts_seq.len() as int));
            assert(t as int + counts_seq[i as int] as int <= u64::MAX as int);
            t = t + self.counts[i];
            i += 1;
        }
        t
    }

    /// Record one selection of `target`: bump its pull count by exactly one,
    /// inserting the target first if unseen. Mirrors
    /// `counts.entry(target).or_insert(0) += 1` in epsilon_greedy.rs / ucb.rs.
    pub fn record_hit(&mut self, target: &str)
        requires
            old(self).wf(),
            old(self).total_spec() < u64::MAX as int,
        ensures
            final(self).wf(),
            final(self).total_spec() == old(self).total_spec() + 1,
            final(self).names@ == old(self).names@ || (final(self).names@.drop_last() == old(self).names@ && final(self).names@.last()@ == target@),
    {
        let ghost total0: int = self.total_spec();
        let ghost counts0: Seq<u64> = self.counts@;
        let ghost names0: Seq<String> = self.names@;
        assert(total0 < u64::MAX as int);
        let mut i: usize = 0;
        let mut done: bool = false;
        while i < self.names.len()
            invariant
                self.names@.len() == self.counts@.len(),
                self.names@ == names0,
                i <= self.names.len(),
                total0 == sum_range(counts0, 0, counts0.len() as int),
                total0 < u64::MAX as int,
                !done ==> self.counts@ == counts0,
                done ==> exists|k: int, v: u64|
                    0 <= k < i && v as int == counts0[k] as int + 1 && #[trigger] self.counts@ == counts0.update(k, v),
            decreases self.names.len() - i
        {
            if !done && self.names[i].as_str() == target {
                proof {
                    lemma_sum_elem_le(counts0, 0, i as int, counts0.len() as int);
                }
                assert(self.counts[i as int] as int <= total0);
                let newc: u64 = self.counts[i] + 1;
                assert(newc as int == counts0[i as int] as int + 1);
                self.counts[i] = newc;
                done = true;
            }
            i += 1;
        }
        if !done {
            self.names.push(target.to_string());
            self.counts.push(1);
            proof {
                lemma_sum_push(counts0, 1);
            }
            assert(self.total_spec() == total0 + 1);
        } else {
            let ghost kv = choose|k: int, v: u64|
                0 <= k < i && v as int == counts0[k] as int + 1 && self.counts@ == counts0.update(k, v);
            let ghost k: int = kv.0;
            let ghost v: u64 = kv.1;
            proof {
                lemma_sum_update_inside(counts0, k, v, 0, counts0.len() as int);
            }
            assert(self.total_spec() == total0 + 1);
        }
    }

    /// Model of a routing session: `n` consecutive recorded selections.
    /// Proves the total pull count equals the number of selection steps and
    /// therefore never exceeds the number of calls.
    pub fn record_hits(&mut self, n: u64, target: &str)
        requires
            old(self).wf(),
            old(self).total_spec() + n as int <= u64::MAX as int,
        ensures
            final(self).wf(),
            final(self).total_spec() == old(self).total_spec() + n as int,
    {
        let ghost total0: int = self.total_spec();
        assert(total0 + n as int <= u64::MAX as int);
        let mut i: u64 = 0;
        while i < n
            invariant
                self.wf(),
                self.total_spec() == total0 + i as int,
                i <= n,
                total0 + n as int <= u64::MAX as int,
            decreases n - i
        {
            self.record_hit(target);
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Model of the per-target success/failure state (thompson.rs).
// ---------------------------------------------------------------------------

pub struct BanditOutcomes {
    pub names: Vec<String>,
    pub successes: Vec<u64>,
    pub failures: Vec<u64>,
}

impl BanditOutcomes {
    pub open spec fn wf(&self) -> bool {
        self.names@.len() == self.successes@.len() && self.names@.len() == self.failures@.len()
    }

    /// Total recorded outcomes = successes + failures, as a mathematical integer.
    pub open spec fn total_spec(&self) -> int {
        sum_range(self.successes@, 0, self.successes@.len() as int) + sum_range(
            self.failures@,
            0,
            self.failures@.len() as int,
        )
    }

    /// Record one outcome: exactly one of the successes/failures counters of
    /// `target` grows, by exactly one. Mirrors `record_reward` in thompson.rs.
    pub fn record_outcome(&mut self, target: &str, success: bool)
        requires
            old(self).wf(),
            old(self).total_spec() < u64::MAX as int,
        ensures
            final(self).wf(),
            final(self).total_spec() == old(self).total_spec() + 1,
            final(self).names@ == old(self).names@ || (final(self).names@.drop_last() == old(self).names@ && final(self).names@.last()@ == target@),
    {
        let ghost total0: int = self.total_spec();
        let ghost succ0: Seq<u64> = self.successes@;
        let ghost fail0: Seq<u64> = self.failures@;
        let ghost names0: Seq<String> = self.names@;
        assert(total0 < u64::MAX as int);
        let mut i: usize = 0;
        let mut done: bool = false;
        while i < self.names.len()
            invariant
                self.names@.len() == self.successes@.len(),
                self.names@.len() == self.failures@.len(),
                self.names@ == names0,
                i <= self.names.len(),
                total0 == sum_range(succ0, 0, succ0.len() as int) + sum_range(fail0, 0, fail0.len() as int),
                total0 < u64::MAX as int,
                !done ==> self.successes@ == succ0 && self.failures@ == fail0,
                done ==> (success ==> exists|k: int, v: u64|
                    0 <= k < i && v as int == succ0[k] as int + 1 && #[trigger] self.successes@ == succ0.update(k, v)
                        && self.failures@ == fail0) && (!success ==> exists|k: int, v: u64|
                    0 <= k < i && v as int == fail0[k] as int + 1 && #[trigger] self.failures@ == fail0.update(k, v)
                        && self.successes@ == succ0),
            decreases self.names.len() - i
        {
            if !done && self.names[i].as_str() == target {
                proof {
                    lemma_sum_elem_le(succ0, 0, i as int, succ0.len() as int);
                    lemma_sum_elem_le(fail0, 0, i as int, fail0.len() as int);
                }
                if success {
                    assert(self.successes[i as int] as int <= total0);
                    let newc: u64 = self.successes[i] + 1;
                    assert(newc as int == succ0[i as int] as int + 1);
                    self.successes[i] = newc;
                } else {
                    assert(self.failures[i as int] as int <= total0);
                    let newc: u64 = self.failures[i] + 1;
                    assert(newc as int == fail0[i as int] as int + 1);
                    self.failures[i] = newc;
                }
                done = true;
            }
            i += 1;
        }
        if !done {
            self.names.push(target.to_string());
            if success {
                self.successes.push(1);
                self.failures.push(0);
                proof {
                    lemma_sum_push(succ0, 1);
                    lemma_sum_push(fail0, 0);
                }
            } else {
                self.successes.push(0);
                self.failures.push(1);
                proof {
                    lemma_sum_push(succ0, 0);
                    lemma_sum_push(fail0, 1);
                }
            }
            assert(self.total_spec() == total0 + 1);
        } else if success {
            let ghost kv = choose|k: int, v: u64|
                0 <= k < i && v as int == succ0[k] as int + 1 && self.successes@ == succ0.update(k, v);
            let ghost k: int = kv.0;
            let ghost v: u64 = kv.1;
            proof {
                lemma_sum_update_inside(succ0, k, v, 0, succ0.len() as int);
            }
            assert(self.total_spec() == total0 + 1);
        } else {
            let ghost kv = choose|k: int, v: u64|
                0 <= k < i && v as int == fail0[k] as int + 1 && self.failures@ == fail0.update(k, v);
            let ghost k: int = kv.0;
            let ghost v: u64 = kv.1;
            proof {
                lemma_sum_update_inside(fail0, k, v, 0, fail0.len() as int);
            }
            assert(self.total_spec() == total0 + 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Opaque float math. The bodies are exactly the formulas of the routing
// algorithms; the integer preconditions are exactly their guards.
// ---------------------------------------------------------------------------

/// Epsilon-greedy average reward `reward_sum / n` (epsilon_greedy.rs:38).
/// The real code only evaluates this division when `n > 0`.
#[verifier::external_body]
pub fn eg_score(reward_sum: f64, n: u64) -> (s: f64)
    requires
        n > 0
{
    reward_sum / (n as f64)
}

/// UCB index `avg + c * sqrt(ln(total) / n)` (ucb.rs:39-45). Both divisions
/// are only reached in the real code when `n > 0`.
#[verifier::external_body]
pub fn ucb_score(reward_sum: f64, c: f64, total: u64, n: u64) -> (s: f64)
    requires
        n > 0
{
    let avg = reward_sum / (n as f64);
    let bonus = c * ((total as f64).ln() / (n as f64)).sqrt();
    avg + bonus
}

/// Positive infinity, used by ucb.rs as the bonus of not-yet-pulled arms.
#[verifier::external_body]
pub fn f64_infinity() -> (i: f64) {
    1.0 / 0.0
}

/// UCB bonus with the real guard `total > 0 && n > 0`; unplayed arms (or an
/// empty history) get infinity instead of the formula (ucb.rs:40-44).
pub fn ucb_bonus_guarded(reward_sum: f64, c: f64, total: u64, n: u64) -> (s: f64) {
    if total > 0 && n > 0 {
        ucb_score(reward_sum, c, total, n)
    } else {
        f64_infinity()
    }
}

/// Epsilon-greedy score with the real guard `n > 0` (epsilon_greedy.rs:38).
pub fn eg_score_guarded(reward_sum: f64, n: u64) -> (s: f64) {
    if n > 0 {
        eg_score(reward_sum, n)
    } else {
        0.0
    }
}

// ---------------------------------------------------------------------------
// Selection. All functions return an index into the registry.
// ---------------------------------------------------------------------------

/// Argmax over precomputed scores. Mirrors the "exploit" loop shared by
/// epsilon_greedy.rs, ucb.rs and thompson.rs (score strictly greater wins).
pub fn argmax(scores: &Vec<f64>) -> (i: usize)
    requires
        scores.len() > 0,
    ensures
        i < scores.len(),
{
    let mut best: usize = 0;
    let mut i: usize = 1;
    while i < scores.len()
        invariant
            1 <= i <= scores.len(),
            best < i,
        decreases scores.len() - i
    {
        if scores[i] > scores[best] {
            best = i;
        }
        i += 1;
    }
    best
}

/// Uniform explore pick `r % len.max(1)` (epsilon_greedy.rs:59, contextual.rs:39).
pub fn explore_pick(r: usize, len: usize) -> (i: usize)
    requires
        len > 0,
    ensures
        i < len,
{
    r % len
}

/// Epsilon-greedy selection: argmax of the average-reward scores, or a
/// uniform pick when exploring. Mirrors the selection of epsilon_greedy.rs;
/// the returned index always names a bandit in the registry.
pub fn select_epsilon_greedy(rewards: &Vec<f64>, counts: &Vec<u64>, explore: bool, r: usize) -> (i: usize)
    requires
        rewards.len() == counts.len(),
        rewards.len() > 0,
    ensures
        i < rewards.len(),
{
    if explore {
        explore_pick(r, rewards.len())
    } else {
        let mut scores: Vec<f64> = Vec::new();
        let mut i: usize = 0;
        while i < rewards.len()
            invariant
                i <= rewards.len(),
                scores.len() == i,
                rewards.len() == counts.len(),
            decreases rewards.len() - i
        {
            scores.push(eg_score_guarded(rewards[i], counts[i]));
            i += 1;
        }
        argmax(&scores)
    }
}

/// UCB selection loop (ucb.rs:36-54). The confidence bonus is only ever
/// computed for arms that have been pulled at least once, and the returned
/// index always names a bandit in the registry.
pub fn select_ucb(
    names: &Vec<String>,
    counts: &Vec<u64>,
    rewards: &Vec<f64>,
    total: u64,
    c: f64,
) -> (r: Option<usize>)
    requires
        names.len() == counts.len(),
        names.len() == rewards.len(),
        total as int == sum_range(counts@, 0, counts@.len() as int),
    ensures
        r is Some ==> (r->0) < names.len(),
{
    if names.len() == 0 {
        return None;
    }
    let mut best: usize = 0;
    let mut best_score: f64 = ucb_bonus_guarded(rewards[0], c, total, counts[0]);
    let mut i: usize = 1;
    while i < names.len()
        invariant
            1 <= i <= names.len(),
            best < i,
            names.len() == counts.len(),
            names.len() == rewards.len(),
        decreases names.len() - i
    {
        let score = ucb_bonus_guarded(rewards[i], c, total, counts[i]);
        if score > best_score {
            best = i;
            best_score = score;
        }
        i += 1;
    }
    Some(best)
}

} // verus!
