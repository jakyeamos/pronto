# Skill usage evidence

Pronto's `pronto-skills/v4` contract fails closed when provider invocation
telemetry is unavailable. Skill discovery, source hashes, provider projection,
hosting, parity, and finding capability remain independently observable; none
of those surfaces imply that a skill was invoked.

## Usage states

- `observed`: Codex's structured local feed identifies successful explicit or
  implicit skill invocation and supplies attributable counts and observation
  time. A zero means no successful invocation has been recorded since the feed
  was installed; it is not a claim about older history.
- `unavailable`: no structured local feed is available. Compatibility counters
  are zeroed, `by_provider` is empty, and consumers must display the state rather
  than interpreting the counters as observed zero usage.

Each usage record includes `telemetry_source` and `reason`. The UI and text CLI
show counts only for `observed` records. Sorting by recent or all-time usage is
available only when at least one skill has observed evidence.

## Privacy and provenance boundary

Pronto does not infer invocations from prompts, assistant messages, injected
skill instructions, available-skill catalogs, filenames, or arbitrary session
text. Those sources cannot distinguish availability, discussion, instruction
loading, and actual invocation. A provider adapter may enable observed usage
only when it supplies a structured invocation event with provider provenance.

Pronto accepts two Codex sources in priority order:

1. The local Codex fork persists explicit and implicit runtime events in the
   `skill_invocations` table of `~/.codex/state_5.sqlite`. Its primary key makes
   a thread/turn/skill/invocation-type event idempotent. Successful events drive
   Pronto's counts and last-seen time; failed explicit loads are retained for
   diagnostics but excluded from usage.
2. Current Codex builds already emit the `codex.skill.injected` OpenTelemetry
   counter with `skill`, `status`, and `invoke_type` dimensions. Pronto's
   localhost-only compatibility collector converts cumulative counter points
   into idempotent deltas in
   `~/Library/Application Support/Pronto/codex-skill-metrics.db`. It records a
   coverage start and heartbeat, so counts never imply coverage before the
   collector was enabled or during a detected interruption.

Neither feed stores prompts, responses, or skill contents. The OTLP adapter
also discards unrelated metrics and resource attributes after hashing the
resource identity needed for counter-series deduplication. Failed skill loads
remain diagnostic records and do not contribute to displayed usage.

Pronto opens the preferred database read-only and aggregates by normalized
skill name. It does not copy thread IDs, turn IDs, paths, or OTLP resource
attributes into the skills snapshot. The compatibility field `all_time_count`
means all events recorded by the selected feed, not reconstructed lifetime
history. Claude, Gemini, Cursor, and pre-collector Codex usage remain
unavailable.

## Current-Codex compatibility collector

After installing the current Pronto bundle, enable the approved persistent
collector with `pnpm skills:collector:install`. It installs the
`com.pronto.skill-usage-collector` user LaunchAgent, binds only
`127.0.0.1:43180`, and adds a marked, reversible `metrics_exporter` entry to
`~/.codex/config.toml`. Restart Codex processes after activation because Codex
loads telemetry configuration at process start.

Codex supports one metrics exporter. While this compatibility route is enabled,
the localhost OTLP exporter replaces Codex's default Statsig metrics exporter.
`pnpm skills:collector:uninstall` removes only Pronto's marked configuration and
LaunchAgent while preserving already recorded local counts. Use
`pnpm skills:collector:check` to verify the configuration and loaded service.

## Migration

Loading a pre-v4 snapshot invalidates any unstructured usage aggregate. Unless
the record is explicitly `observed` and names a non-empty structured telemetry
source, Pronto clears its counts, provider breakdown, and last-seen timestamp and
returns the v4 unavailable state. Observed counts must also be non-negative
integers, recent counts cannot exceed recorded counts, provider totals must
match the aggregate, and a nonzero aggregate requires a valid observation
timestamp. Both the Rust and renderer boundaries enforce these invariants. A
skills refresh persists the v4 contract.
