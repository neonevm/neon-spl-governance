#!/bin/bash
set -euo pipefail

set ${SOLANA_REVISION:=v1.9.12}

while getopts t: option; do
case "${option}" in
    t) IMAGETAG=${OPTARG};;
    *) echo "Usage: $0 [OPTIONS]. Where OPTIONS can be:"
       echo "    -t <IMAGETAG>  tag for neonlabsorg/neon-governance Docker-image"
       exit 1;;
esac
done

export NEON_GOVERNANCE_IMAGE=neonlabsorg/neon-governance:${IMAGETAG:-${BUILDKITE_COMMIT}}

echo "Currently running Docker-containers"
docker ps -a

function cleanup_docker {
    docker logs solana >solana.log 2>&1
    echo "Cleanup docker-compose..."
    docker-compose -f docker-compose-test.yml down --timeout 1
    echo "Cleanup docker-compose done."
}
trap cleanup_docker EXIT

echo "\nCleanup docker-compose..."
docker-compose -f docker-compose-test.yml down --timeout 1

if ! docker-compose -f docker-compose-test.yml up -d; then
    echo "docker-compose failed to start"
    exit 1;
fi

# waiting for solana to launch
sleep 10

echo "Run tests..."
docker run --rm --network neon-governance-run-test-net -ti \
     -e SOLANA_URL=http://solana:8899 \
     ${EXTRA_ARGS:-} \
     $NEON_GOVERNANCE_IMAGE '/opt/run-tests.sh'
#docker exec -ti solana '/opt/run-tests.sh'
echo "Run tests return"

exit $?
