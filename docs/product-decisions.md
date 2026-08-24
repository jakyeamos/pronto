# Pronto product decisions

Recorded 2026-07-25 after review of `/Users/jakyeamos/Downloads/pronto_prd.md`.

These decisions define the next implementation boundary. They are intentionally conservative: local evidence remains useful without requiring an account, network access, or operational authority.

| Decision            | Accepted direction                                                                | Implementation consequence                                                                                                                                                                                                                 |
| ------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Delivery order      | Polish the local desktop experience before provider breadth.                      | Improve navigation, freshness, empty states, settings, and keyboard access first.                                                                                                                                                          |
| Durable state       | Move to SQLite before adding provider data.                                       | Preserve the domain contracts while replacing the JSON persistence layer with a migration-safe local database.                                                                                                                             |
| GitHub              | Read-only identity and repository/PR context, plus one guarded visibility action. | Repository visibility changes require fresh administration preflight, exact confirmation, provider readback, and a durable local audit; no automatic PR creation, merge, release, or publish behavior.                                     |
| Actions             | Start with safe local actions and narrowly bounded provider writes.               | Read-only refresh/inspect preflights and the GitHub visibility action record exact targets and durable outcomes; open, workspace operations, and all destructive/history-rewriting Git operations remain behind explicit future contracts. |
| Platform            | macOS first.                                                                      | Verify the live Tauri shell, folder permissions, and signed packaging before expanding the support matrix.                                                                                                                                 |
| AI                  | Disabled by default.                                                              | No model, credential, source-content, or operational-decision path is added until data-sharing and authority are explicitly designed.                                                                                                      |
| Products and groups | Manual configuration first.                                                       | User-defined structure is authoritative; inference is a later convenience, never a silent replacement.                                                                                                                                     |

## Next sequence

1. Finish the local UX pass and verify it against the existing local evidence contract. (Complete.)
2. Replace the JSON registry with versioned SQLite persistence while preserving CLI and renderer snapshot shapes. (Complete, including legacy JSON migration.)
3. Add safe local action preflight and audit records without enabling destructive operations. (Complete for read-only refresh/inspect.)
4. Add GitHub identity and remote context, with repository visibility as the only approval-gated provider write; keep PR, merge, release, and publish actions deferred.
