const { sessionBadge } = require("./session");

const session = { userId: "developer" };
if (sessionBadge(session) !== "developer · active") {
  throw new Error(
    "sessionBadge should make active work recognizable at a glance",
  );
}

console.log("focused session badge assertion passed");
