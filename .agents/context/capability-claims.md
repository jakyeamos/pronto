# Capability claims gate

Last reviewed: 2026-08-17.

Use this gate before telling a user that a tool, connector, skill, or other
agent capability is unavailable. A capability claim is evidence-bearing state,
not a guess based on the current surface, thread type, or a nearby restriction.

## Required sequence

1. Inspect the currently exposed tool surface or provider catalog. Use the
   runtime tool list, tool search, or another authoritative read-only discovery
   path available in the current turn.
2. Attempt the smallest safe read-only discovery call that can distinguish an
   absent capability from a blocked or unauthorized operation.
3. Classify the result using the contract in
   [`capability-gate.json`](capability-gate.json):

   - `unavailable`: discovery completed and no callable capability was exposed.
   - `prohibited`: the capability is exposed, but policy forbids using it.
   - `authorization_required`: the capability is exposed, but this operation
     needs authority that has not been granted.
   - `available_authorized`: the capability is exposed, in scope, and callable.
   - `discovery_blocked`: discovery could not complete; availability is
     unknown, not unavailable.

4. Keep the evidence with the claim: name the tool-surface source, discovery
   result, and scope or policy basis.

## Boundary rule

A narrow prohibition does not generalize to neighboring capabilities. For
example, a side-conversation rule that prohibits sub-agent interaction does
not by itself prove that Codex app thread messaging is unavailable. Inspect the
thread-messaging tool surface separately and classify its authority for the
requested operation.

Never report `unavailable` when the only evidence is a missing search, an
unread tool catalog, a policy restriction on a different capability, or an
unverified assumption about the current thread.
