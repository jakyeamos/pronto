# Papercuts

Papercuts are the durable backlog surface for the design-audit family.

The boundary is intentional:

- `design-friction-audit` is a per-turn sensor. It can point out a repeated
  small hurt while answering a request, but it does not write project state.
- **Papercuts** is a built-in skill in Pronto's **Skills** surface. Selecting
  the skill opens the explicit capture point, where a person records the
  symptom, surface, evidence references, impact, priority, and next validation
  step. The same skill is addressable through `pronto skills papercuts`.
- The backlog is stored locally in Pronto under the
  `pronto-papercuts/v1` schema. Items can be `open`, `in_progress`,
  `deferred`, or `resolved`; resolved items remain visible as audit history.
- The **Promotion inbox** remains a separate AWL → JAS handoff. Papercuts are
  not promotion candidates and are not automatically sent to another system.

The UI does not ingest prompt transcripts automatically. Repeated friction
becomes durable only through an explicit capture in the Papercuts skill detail
surface.
