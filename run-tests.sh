#!/bin/bash
set -euo pipefail

solana config set --url ${SOLANA_URL:-http://localhost:8899}

./init-governance.sh


# INTEGRATION TEST FOR GOVERNANCE OPERATION

# Address for NEON realm (PDA for spl-governance with seeds:['governance',name='NEON'])
# Can be calculated like this:
# ```
# SPL_GOVERNANCE_ID=$(solana address -k artifacts/spl-governance.keypair)
# NEON_REALM=$(python3 -c "from solana.publickey import PublicKey; print(PublicKey.find_program_address([b'governance',b'NEON'], PublicKey('$SPL_GOVERNANCE_ID'))[0])")
# ```
NEON_REALM=HQ2gGKpAqFHoUWViJNHa8ARTiwBGisMDDrL2A8q4WiiC

# Stage 0: Preparing Governance subsystem (all contracts already loaded in ./init-governance.sh step)
launch-script --config testing.cfg --send-trx environment dao
launch-script --config testing.cfg --send-trx proposal --name 'Delegate vote to payer' --governance MSIG_4.1 create-delegate-vote --delegate $(solana address) --realm $NEON_REALM
launch-script --config testing.cfg --send-trx proposal --governance MSIG_4.1 --proposal LAST sign-off
launch-script --config testing.cfg --send-trx proposal --governance MSIG_4.1 --proposal LAST approve --voters artifacts/voters/
launch-script --config testing.cfg --send-trx proposal --governance MSIG_4.1 --proposal LAST execute

# Stage 1: Preparing Token Genesis Event and switch to vesting-addin
launch-script --config testing.cfg --send-trx proposal --name 'Token Genesis Event' create-tge
launch-script --config testing.cfg --send-trx proposal --proposal LAST sign-off
launch-script --config testing.cfg --send-trx proposal --proposal LAST approve --voters artifacts/voters/
sleep 185
launch-script --config testing.cfg --send-trx proposal --proposal LAST finalize-vote
sleep 65
launch-script --config testing.cfg --send-trx proposal --proposal LAST execute
