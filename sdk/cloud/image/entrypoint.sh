#!/bin/bash
# raea cloud's entrypoint (docs/cloud.feature CL3, CL10): the home folder back from RAEA_CLOUD_STATE when it's set,
# raea's skills for each installed agent (matched to this image's raea), then the server. On SIGTERM (a platform
# stopping or sleeping the container) the server stops and the home folder is saved again.
set -uo pipefail
here=/opt/raea-cloud
: "${RAEA_CLOUD_TOKEN:?raea-cloud: set RAEA_CLOUD_TOKEN (openssl rand -hex 32); every request must carry it}"

# started as root: give the home folder to raea (a new volume is root's) and go on as raea, environment kept
if [ "$(id -u)" = 0 ]; then
  [ "$(stat -c %u /home/raea)" = 1000 ] || chown -R raea:raea /home/raea
  exec setpriv --reuid=raea --regid=raea --init-groups "$0" "$@"
fi

"$here/image/state.sh" restore
mkdir -p "${RAEA_CLOUD_WORK:-$HOME/work}"

for a in $(node "$here/server/agents.ts" skills); do
  raea skills install --agent "$a" >/dev/null 2>&1 || echo "raea-cloud: raea's skills for $a weren't installed"
done

node "$here/server/main.ts" &
server=$!
stop() {
  kill -TERM "$server" 2>/dev/null
  wait "$server" 2>/dev/null
  "$here/image/state.sh" save
  exit 0
}
trap stop TERM INT
wait "$server"
