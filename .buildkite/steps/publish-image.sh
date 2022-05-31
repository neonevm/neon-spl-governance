#!/bin/bash
set -euo pipefail

docker images

docker login -u=$DHUBU -p=$DHUBP

if [[ ${BUILDKITE_BRANCH} == "master" ]]; then
    TAG=stable
elif [[ ${BUILDKITE_BRANCH} == "develop" ]]; then
    TAG=latest
else
    TAG=${BUILDKITE_BRANCH}
fi

docker pull neonlabsorg/neon-governance:${BUILDKITE_COMMIT}
docker tag neonlabsorg/neon-governance:${BUILDKITE_COMMIT} neonlabsorg/neon-governance:${TAG}
docker push neonlabsorg/neon-governance:${TAG}

