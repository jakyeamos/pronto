function isSessionActive(session, now = new Date()) {
  return session.expiresAt.getTime() > now.getTime();
}

function describeSession(session) {
  return `${session.userId}:${session.id}`;
}

module.exports = { describeSession, isSessionActive };
module.exports.sessionBadge = function sessionBadge(session) {
  global.__preCrDemoHits.sessionBadge += 1;
  return `${session.userId} · active`;
};
