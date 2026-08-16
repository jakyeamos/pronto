import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";
import {
  codexOtlpMarkers,
  configureCodexOtlp,
  launchAgentPlist,
  removeCodexOtlp,
} from "./skill-usage-collector-lib.mjs";

const label = "com.pronto.skill-usage-collector";
const endpoint = "http://127.0.0.1:43180/v1/metrics";
const userId = process.getuid?.();
if (userId == null) throw new Error("A macOS user ID is required.");
const domain = `gui/${userId}`;
const home = homedir();
const configPath = join(home, ".codex", "config.toml");
const executable = "/Applications/Pronto.app/Contents/MacOS/pronto";
const plistPath = join(home, "Library", "LaunchAgents", `${label}.plist`);
const logDirectory = join(home, "Library", "Logs", "Pronto");
const logPath = join(logDirectory, "skill-usage-collector.log");
const errorLogPath = join(logDirectory, "skill-usage-collector.error.log");

function writeAtomic(path, contents) {
  mkdirSync(dirname(path), { recursive: true });
  const temporary = `${path}.tmp-${process.pid}`;
  writeFileSync(temporary, contents, { mode: 0o600 });
  renameSync(temporary, path);
}

function launchctl(...args) {
  return execFileSync("/bin/launchctl", args, { encoding: "utf8" });
}

function bootoutIfLoaded() {
  try {
    launchctl("bootout", domain, plistPath);
  } catch {
    // An absent service is already in the desired pre-bootstrap state.
  }
}

function install() {
  if (!existsSync(executable)) {
    throw new Error(
      `${executable} is missing. Build and install Pronto before enabling the collector.`,
    );
  }
  if (!existsSync(configPath)) {
    throw new Error(`Codex configuration is missing at ${configPath}.`);
  }
  const currentConfig = readFileSync(configPath, "utf8");
  const nextConfig = configureCodexOtlp(currentConfig, endpoint);
  if (nextConfig !== currentConfig) writeAtomic(configPath, nextConfig);

  mkdirSync(logDirectory, { recursive: true });
  const plist = launchAgentPlist({ executable, logPath, errorLogPath });
  bootoutIfLoaded();
  writeAtomic(plistPath, plist);
  launchctl("bootstrap", domain, plistPath);
  launchctl("kickstart", "-k", `${domain}/${label}`);
  check();
  process.stdout.write(
    `Pronto skill usage collector enabled at ${endpoint}. Restart Codex processes to load the exporter.\n`,
  );
}

function check() {
  if (!existsSync(plistPath))
    throw new Error(`LaunchAgent is missing at ${plistPath}.`);
  if (!existsSync(configPath))
    throw new Error(`Codex configuration is missing at ${configPath}.`);
  const config = readFileSync(configPath, "utf8");
  if (
    !config.includes(codexOtlpMarkers.markerStart) ||
    !config.includes(endpoint)
  ) {
    throw new Error(
      "Codex is not configured for Pronto's local OTLP collector.",
    );
  }
  launchctl("print", `${domain}/${label}`);
  process.stdout.write(
    "Pronto skill usage collector configuration is installed and loaded.\n",
  );
}

function uninstall() {
  bootoutIfLoaded();
  if (existsSync(plistPath)) unlinkSync(plistPath);
  if (existsSync(configPath)) {
    const currentConfig = readFileSync(configPath, "utf8");
    const nextConfig = removeCodexOtlp(currentConfig);
    if (nextConfig !== currentConfig) writeAtomic(configPath, nextConfig);
  }
  process.stdout.write(
    "Pronto skill usage collector disabled; recorded local counts were preserved.\n",
  );
}

const action = process.argv[2] ?? "--install";
if (action === "--install") install();
else if (action === "--check") check();
else if (action === "--uninstall") uninstall();
else throw new Error(`Unknown action: ${action}`);
