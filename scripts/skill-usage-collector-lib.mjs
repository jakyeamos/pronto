const markerStart = "# BEGIN PRONTO CODEX SKILL USAGE";
const markerEnd = "# END PRONTO CODEX SKILL USAGE";

function exporterLine(endpoint) {
  return `metrics_exporter = { otlp-http = { endpoint = "${endpoint}", protocol = "json" } }`;
}

function sectionBounds(lines, section) {
  const start = lines.findIndex((line) => line.trim() === `[${section}]`);
  if (start < 0) return null;
  const next = lines.findIndex(
    (line, index) => index > start && /^\s*\[[^[]/.test(line),
  );
  return { start, end: next < 0 ? lines.length : next };
}

export function configureCodexOtlp(contents, endpoint) {
  if (contents.includes(markerStart) || contents.includes(markerEnd)) {
    if (contents.includes(exporterLine(endpoint))) return contents;
    throw new Error("Pronto's existing Codex OTLP block is inconsistent.");
  }
  const lines = contents.replace(/\r\n/g, "\n").split("\n");
  const otel = sectionBounds(lines, "otel");
  if (otel) {
    const existingExporter = lines
      .slice(otel.start + 1, otel.end)
      .find((line) => /^\s*metrics_exporter\s*=/.test(line));
    if (existingExporter) {
      throw new Error(
        "Codex already has an [otel] metrics_exporter; Pronto will not overwrite it.",
      );
    }
    lines.splice(
      otel.start + 1,
      0,
      markerStart,
      "# Local-only compatibility feed; replaces the default metrics exporter while enabled.",
      exporterLine(endpoint),
      markerEnd,
    );
  } else {
    while (lines.at(-1) === "") lines.pop();
    lines.push(
      "",
      markerStart,
      "[otel]",
      "# Local-only compatibility feed; replaces the default metrics exporter while enabled.",
      exporterLine(endpoint),
      markerEnd,
      "",
    );
  }
  return lines.join("\n");
}

export function removeCodexOtlp(contents) {
  const lines = contents.replace(/\r\n/g, "\n").split("\n");
  const start = lines.findIndex((line) => line.trim() === markerStart);
  const end = lines.findIndex(
    (line, index) => index >= start && line.trim() === markerEnd,
  );
  if (start < 0 && end < 0) return contents;
  if (start < 0 || end < start) {
    throw new Error(
      "Pronto's Codex OTLP configuration markers are incomplete.",
    );
  }
  const includesSection = lines
    .slice(start, end + 1)
    .some((line) => line.trim() === "[otel]");
  lines.splice(start, end - start + 1);
  if (includesSection) {
    while (start < lines.length && lines[start] === "") lines.splice(start, 1);
  }
  return lines.join("\n");
}

export function launchAgentPlist({ executable, logPath, errorLogPath }) {
  const escapeXml = (value) =>
    value
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&apos;");
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.pronto.skill-usage-collector</string>
  <key>ProgramArguments</key>
  <array>
    <string>${escapeXml(executable)}</string>
    <string>--skill-usage-collector</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>ProcessType</key>
  <string>Background</string>
  <key>ThrottleInterval</key>
  <integer>10</integer>
  <key>StandardOutPath</key>
  <string>${escapeXml(logPath)}</string>
  <key>StandardErrorPath</key>
  <string>${escapeXml(errorLogPath)}</string>
</dict>
</plist>
`;
}

export function launchAgentRegistrationPlan({
  loaded,
  currentPlist,
  desiredPlist,
}) {
  if (loaded && currentPlist === desiredPlist) return "restart";
  if (loaded) return "reregister";
  return "bootstrap";
}

export const codexOtlpMarkers = { markerStart, markerEnd };
