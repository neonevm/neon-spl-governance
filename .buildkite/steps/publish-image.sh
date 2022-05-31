#!/bin/bash
set -euo pipefail

docker images

docker login -u=$DHUBU -p=$DHUBP

if [[ ${BUILDKITE_BRANCH} == "main" ]]; then
    TAG=stable
else
    TAG=${BUILDKITE_BRANCH}
fi

docker pull neonlabsorg/neon-governance:${BUILDKITE_COMMIT}
docker tag neonlabsorg/neon-governance:${BUILDKITE_COMMIT} neonlabsorg/neon-governance:${TAG}
docker push neonlabsorg/neon-governance:${TAG}

