import fs from "node:fs";
import path from "node:path";

const coverageDirectory = path.join(globalThis.process.cwd(), ".pre-cr");
fs.mkdirSync(coverageDirectory, { recursive: true });
fs.writeFileSync(
  path.join(coverageDirectory, "coverage.lcov"),
  "TN: no-instrumentation\nend_of_record\n",
  "utf8",
);
