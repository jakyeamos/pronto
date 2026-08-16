# Evidence contract freshness

Pronto tracks two independent freshness dimensions for imported evidence:

- observation freshness answers whether the evidence was collected recently;
- contract freshness answers whether every scoped repository was assessed
  against the current evidence obligations.

A fresh observation can therefore still require a full fleet audit. This is
expected when an evidence producer adds a lane, field, oracle, or other
obligation that old reports could not have assessed.

## Generic projection

Evidence owners register a stable `contract_id`, a user-facing `label`, and a
current `target_schema`. Each repository reports its `observed_schema`. Pronto
projects the result into the generic `quality.evidence_contracts` collection:

- `current`: the observed schema exactly matches the target;
- `audit_required`: evidence is readable but was produced for another schema;
- `missing`: no schema was recorded.

The portfolio projection aggregates current, legacy, and missing repository
counts. Any non-current repository produces a **Full fleet audit required**
banner, repository attention item, and generic remediation action. Legacy
evidence remains visible for diagnosis but cannot satisfy a current ideal-state
claim.

The evidence owner must bump its schema whenever the evidence obligations
change. Compatibility parsing is not contract currency: accepting an older
report for readback must never silently promote it to `current`.

## First registered contract

Mac Control task evidence is the first producer:

- contract ID: `mac-control-task-manifest`
- current schema: `mac-control-task-manifest/v3`
- owner: the Mac Control / Quality Runner fleet-audit producer

The v3 contract adds shortcut acceleration evidence. v1 and v2 reports remain
readable, but every scoped repository must be re-audited before the fleet can
return to current status, including repositories whose applicability is
`not_applicable` because that disposition must be re-attested under the new
contract.

Future evidence lanes should register through this same projection rather than
adding product-specific freshness banners or remediation logic.
