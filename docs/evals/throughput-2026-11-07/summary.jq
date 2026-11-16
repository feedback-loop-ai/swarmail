# Summary of an admitted mail_compare report (protocol v2, report version 2).
# Input: report.json. `mail_compare admit` has already proved every cell
# read here to be the arithmetic of samples.jsonl; this program only
# groups the per-repetition cells and adds nothing the report lacks.
#
#   jq -S -f summary.jq report.json          > summary.json
#   jq -c --argjson runs true -f summary.jq report.json > runs.jsonl
#
# Median over repetitions: the middle value, or the mean of the two middle
# values when the count is even (six repetitions here).
def median: sort | length as $n
  | if $n == 0 then null
    elif $n % 2 == 1 then .[($n - 1) / 2]
    else (.[$n / 2 - 1] + .[$n / 2]) / 2 end;
def spread: map(select(. != null)) as $v
  | {median: ($v | median), min: ($v | min), max: ($v | max), of: ($v | length)};
def q($p): .values | map(select(.[0] == $p)) | first | .[1];
def profiles: ["throughput-c1-fresh", "throughput-c10-fresh", "throughput-c100-fresh",
  "throughput-c1-reused", "throughput-c10-reused", "throughput-c100-reused",
  "stress-c1000-fresh"];
def targets: ["swarmail", "mailpit", "mailhog"];
def rank($xs): . as $x | $xs | index($x);
def run_row: {
  target, profile, repetition, requested, admitted, refused, unstarted,
  ambiguous: .reconciliation.ambiguous,
  visible_unique: .reconciliation.visible_unique,
  missing_accepted: .reconciliation.missing_accepted,
  duplicates: .reconciliation.duplicates,
  fidelity_mismatches: .reconciliation.fidelity_mismatches,
  exact,
  send_window_ns: .reconciliation.send_window_ns,
  verified_window_ns: .reconciliation.verified_window_ns,
  accepted_per_second, verified_per_second,
  send_latency_ns: {p50: (.send_latency | q(50)), p95: (.send_latency | q(95)),
    p99: (.send_latency | q(99)), samples: .send_latency.samples}
};

if ($ARGS.named.runs // false) then
  .throughput
  | sort_by([(.profile | rank(profiles)), (.target | rank(targets)), .repetition])
  | .[] | run_row
else
  {
    run_id, protocol_id, protocol_sha256, evidence,
    claim_scope: .claim.scope, storage_profile, resource_profile,
    median_rule: "middle value, or the mean of the two middle values for an even count, over repetitions",
    throughput: (
      .throughput | group_by([(.profile | rank(profiles)), (.target | rank(targets))])
      | map({
          profile: .[0].profile, target: .[0].target, sessions: .[0].sessions,
          repetitions: length,
          exact_repetitions: map(select(.exact)) | length,
          requested: map(.requested) | add,
          admitted: map(.admitted) | add,
          refused: map(.refused) | add,
          unstarted: map(.unstarted) | add,
          ambiguous: map(.reconciliation.ambiguous) | add,
          missing_accepted: map(.reconciliation.missing_accepted) | add,
          fidelity_mismatches: map(.reconciliation.fidelity_mismatches) | add,
          verified_per_second: (map(.verified_per_second) | spread),
          accepted_per_second: (map(.accepted_per_second) | spread),
          send_latency_p50_ns: (map(.send_latency | q(50)) | spread)
        })
    ),
    waits: (
      .waits | sort_by(.target | rank(targets)) | map({
        target, profile, repetitions, requests, outcomes,
        present_ns: {p50: (.present_latency | q(50)), p95: (.present_latency | q(95)),
          p99: (.present_latency | q(99)), samples: .present_latency.samples,
          censored: .present_latency.censored},
        future_ns: {p50: (.future_latency | q(50)), p95: (.future_latency | q(95)),
          p99: (.future_latency | q(99)), samples: .future_latency.samples,
          censored: .future_latency.censored},
        future_offset_from_ack_ns: {p50: (.future_offset_from_ack | q(50)),
          p95: (.future_offset_from_ack | q(95)), p99: (.future_offset_from_ack | q(99))}
      })
    )
  }
end
