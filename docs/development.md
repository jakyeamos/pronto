# Development workflow

Use the live desktop development command while changing code:

```sh
pnpm dev
```

That launches the current checkout with Tauri's live reload. It does not use
the copy in `/Applications`.

When you need to test the installed macOS app, use the single release-like
command:

```sh
pnpm app
```

It builds the current checkout, copies the resulting `Pronto.app` to
`/Applications/Pronto.app`, and verifies that the entire installed app bundle
and native executable match the build. Quit and reopen Pronto after installing
if it was already running.

To check for install drift without rebuilding:

```sh
pnpm app:check
```

`pnpm build` still only creates the release bundle inside the repository. It
does not update `/Applications`, and none of these commands modify the local
Pronto database.
