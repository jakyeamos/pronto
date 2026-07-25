# Pronto product decisions

Recorded 2026-07-25 after review of `/Users/jakyeamos/Downloads/pronto_prd.md`.

These decisions define the next implementation boundary. They are intentionally conservative: local evidence remains useful without requiring an account, network access, or operational authority.

| Decision            | Accepted direction                                           | Implementation consequence                                                                                                                      |
| ------------------- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Delivery order      | Polish the local desktop experience before provider breadth. | Improve navigation, freshness, empty states, settings, and keyboard access first.                                                               |
| Durable state       | Move to SQLite before adding provider data.                  | Preserve the domain contracts while replacing the JSON persistence layer with a migration-safe local database.                                  |
| GitHub              | Read-only identity and repository/PR context initially.      | No write scopes, automatic PR creation, merge, release, or publish behavior.                                                                    |
| Actions             | Start with safe, non-destructive local actions.              | Refresh, inspect, open, and bounded workspace operations may be considered; destructive and history-rewriting Git operations remain prohibited. |
| Platform            | macOS first.                                                 | Verify the live Tauri shell, folder permissions, and signed packaging before expanding the support matrix.                                      |
| AI                  | Disabled by default.                                         | No model, credential, source-content, or operational-decision path is added until data-sharing and authority are explicitly designed.           |
| Products and groups | Manual configuration first.                                  | User-defined structure is authoritative; inference is a later convenience, never a silent replacement.                                          |

## Next sequence

1. Finish the local UX pass and verify it against the existing local evidence contract.
2. Replace the JSON registry with versioned SQLite persistence while preserving CLI and renderer snapshot shapes.
3. Add safe local action preflight and audit records without enabling destructive operations.
4. Add read-only GitHub identity and remote context only after the durable-state and permission boundaries are verified.
