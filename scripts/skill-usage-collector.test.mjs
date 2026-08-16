import assert from "node:assert/strict";
import test from "node:test";
import {
  configureCodexOtlp,
  launchAgentPlist,
  removeCodexOtlp,
} from "./skill-usage-collector-lib.mjs";

const endpoint = "http://127.0.0.1:43180/v1/metrics";

test("adds and removes an isolated otel section", () => {
  const original = 'model = "gpt-5"\n';
  const configured = configureCodexOtlp(original, endpoint);
  assert.match(configured, /\[otel\]/);
  assert.match(configured, /127\.0\.0\.1:43180/);
  assert.equal(removeCodexOtlp(configured).trim(), original.trim());
});

test("adds a marked exporter without replacing an existing otel section", () => {
  const original = '[otel]\nenvironment = "dev"\n\n[features]\napps = true\n';
  const configured = configureCodexOtlp(original, endpoint);
  assert.match(configured, /\[otel\]\n# BEGIN PRONTO/);
  assert.match(configured, /environment = "dev"/);
  assert.equal(removeCodexOtlp(configured), original);
});

test("refuses to overwrite a user-owned exporter", () => {
  assert.throws(
    () => configureCodexOtlp('[otel]\nmetrics_exporter = "none"\n', endpoint),
    /will not overwrite/,
  );
});

test("launch agent is loopback collector only", () => {
  const plist = launchAgentPlist({
    executable: "/Applications/Pronto.app/Contents/MacOS/pronto",
    logPath: "/tmp/pronto.log",
    errorLogPath: "/tmp/pronto.error.log",
  });
  assert.match(plist, /com\.pronto\.skill-usage-collector/);
  assert.match(plist, /--skill-usage-collector/);
  assert.match(plist, /<key>KeepAlive<\/key>/);
});
