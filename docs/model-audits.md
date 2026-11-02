# Refreshable model audit projection

Pronto projects Quality Runner's retained model-audit evidence. QR prepares
the inputs, accepts model responses and separate verification, and calculates
the comparison. Pronto does not run models, fix code, or create another findings
database.

Use a QR build with `qr model-audit --help` available. The standalone bridge
accepts an explicit executable; unsupported producers fail visibly.

```sh
node bin/pronto.mjs model-audit report /absolute/repository --run-id audit-002 \
  --qr-bin /absolute/quality-runner/.venv/bin/qr --json
node bin/pronto.mjs model-audit fleet --selection /absolute/selection.json \
  --qr-bin /absolute/quality-runner/.venv/bin/qr --json
```

The fleet selection is a local JSON object:

```json
{
  "schema": "quality-runner-model-audit-selection/v1",
  "repositories": [
    { "repo_path": "/absolute/repository", "run_id": "audit-002" }
  ]
}
```

Read the producer's coverage, comparison controls and per-finding status.
Newly verified, known, disputed, unverified and not reobserved remain distinct.
Omission never means resolved. Missing evidence remains unavailable; partial
fleet reports retain their JSON and exit 1. A wrong schema, mismatched
repository/run, or implementation authority is rejected. For fleet readback,
Pronto independently binds all reported and unavailable pairs to the saved
selection, rejects duplicate or omitted pairs, and checks the coverage totals.
Each nested report must also preserve read-only authority.

This is an explicit CLI projection. It does not import model findings into the
desktop quality score or remediation queue. Model-audit run manifests are
excluded from the legacy latest deterministic QR run selector so they cannot
displace gate evidence. The generic all-command run roll-up described by newer
global guidance is unavailable on this integration baseline; the explicit
bridge supplies the supported readback here.

For a new model, prepare a fresh QR audit with the saved profile, new reviewer
settings, `--baseline-run-id` and `--reuse-inputs-from` for a fixed-source
comparison. For current-code discovery, omit input reuse. QR's
`docs/model-audits.md` owns the full producer and verification contract.
Reports describe retained snapshots; current workspace freshness is not checked.
