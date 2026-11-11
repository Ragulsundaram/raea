#!/bin/bash
# raea cloud's end-to-end smoke test (docs/cloud.feature CL12): builds the image (unless IMAGE names one), starts it
# with test/echo-agent.mjs as its agent, runs test/smoke.ts against it, and removes the container and its volume.
#
#   test/smoke.sh [RAEA_VERSION]          default: the version in package.json
#   IMAGE=raea-cloud:dev test/smoke.sh    an image already built
set -uo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
v=${1:-$(node -p "require('$here/package.json').version")}
image=${IMAGE:-}
if [ -z "$image" ]; then
  image=raea-cloud:smoke
  docker build --platform linux/amd64 -f "$here/image/Dockerfile" --build-arg RAEA_VERSION="$v" --build-arg AGENTS="${AGENTS:-kimi-code}" -t "$image" "$here" || exit 1
fi
name=raea-cloud-smoke-$$
token=$(openssl rand -hex 32)
port=${PORT:-18080}
cleanup() { docker rm -f "$name" >/dev/null 2>&1; docker volume rm "$name-home" >/dev/null 2>&1; }
trap cleanup EXIT
docker run -d --platform linux/amd64 --name "$name" -p "127.0.0.1:$port:8080" -v "$name-home:/home/raea" \
  -v "$here/test:/opt/raea-cloud/test:ro" -e RAEA_CLOUD_TOKEN="$token" \
  -e RAEA_AGENT=custom -e RAEA_AGENT_CMD="node /opt/raea-cloud/test/echo-agent.mjs" "$image" >/dev/null || exit 1
for _ in $(seq 1 120); do
  curl -fsS "http://127.0.0.1:$port/healthz" >/dev/null 2>&1 && break
  docker inspect -f '{{.State.Running}}' "$name" 2>/dev/null | grep -q true || { docker logs "$name"; exit 1; }
  perl -e 'select(undef,undef,undef,0.5)'
done
# a symlink in the workspace that points out of it, for the path check
docker exec "$name" sh -c 'ln -sfn /etc "$HOME/work/out-link"'
node "$here/test/smoke.ts" "http://127.0.0.1:$port" "$token"
code=$?
echo "--- container log"; docker logs "$name" 2>&1 | tail -15
exit $code
