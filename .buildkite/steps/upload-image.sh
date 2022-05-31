#!/bin/bash
set -euo pipefail

docker images

docker login -u=${DHUBU} -p=${DHUBP}

docker push neonlabsorg/neon-governance:${BUILDKITE_COMMIT}


