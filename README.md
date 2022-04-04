# neon-spl-governance
 
## External weights for spl-governance

This code uses the possibility of the spl-governance contract to obtain the weight
of the user's vote from an external source. Initially, this feature was implemented
in version 2.1.0 (commit c99f4195f1295afad96e3af5cb1199554150b7b0,
https://github.com/solana-labs/solana-program-library/pull/2450) and has undergone
some changes in subsequent versions. The current version of the solana-program-library
is linked as a submodule of this repository.
The documentation for the spl-governance contract is in its repository:
https://github.com/solana-labs/solana-program-library/tree/master/governance

This feature allows you to redefine the way to determine the number of votes a voter
can vote with (and perform a number of other actions) in a spl-governance program Realm.
This is done by creating an addin program that generates special records
`VoterWeightRecord`/`MaxVoterWeightRecord` with a fixed internal structure (see 
governance/addin-api inside the solana-program-library repository).

If a Realm settings specify usage of an addin program to obtain a weight of a voter
or a total weight of all voters (these settings are specified independently), then
the spl-governance program uses records belonging to a current voter to obtain voter weight.


## addin-fixed-weights

The addin is used to create decentralized management UNTIL the moment of token issuance.
Implements the Voter weight addin interface to create both a `VoterWeightRecord`
and a `MaxVoterWeightRecord`. Since there are no tokens yet, Realm configuration requires
usage of `MaxVoterWeightRecord`.

User addresses, as well as their voting weights, are set during compilation and cannot
be changed later (without updating the contract). This contract is actually intended
only for creating voter weight records with fixed content that can be used by spl-governance.

To switch a Realm to the operation with the actual voter weights received after
the issuance of tokens, it is assumed to change the Realm settings and specify another
addin. After changing the settings, all records created by this addin will be
considered invalid by the spl-governance program (therefore, for correct Realm operation
after disabling this addin, deleting these records is optional).


## addin-vesting

Implements functionality similar to the basic deposit/withdraw functionality inside
the spl-governance program to connect a user to voting capabilities.
In addition to this, implements:
- the ability to make a schedule for the withdrawal of the locked tokens in whole or
in parts at certain points in time;
- the ability to vote by a part of user's locked tokens (since this feature has not
yet been implemented under the spl-governance contract).

Token deposit is possible in one of two modes (selected independently for each
record being created):
- vesting only;
- vesting for Realm


### Vesting Only

It is intended for storing tokens and release in accordance with a certain schedule.
This schedule allows you to withdraw tokens in parts.
The following contract instructions are intended for operation:
- `Deposit` (basic fields) - deposit of tokens with defined recipient and withdrawal schedule;
- `Withdraw` (basic fields) - withdrawal of tokens by a recipient;
- `ChangeOwner` (base fields) - change a recipient of a vesting.

When withdrawing tokens, it is checked that the moment for their release has come.


### Vesting for Realm

Designed to connect users to the voting features in a certain Realm. A user obtains
the weight of a vote in accordance with the amount of tokens deposited (in total for
all vesting records belonging to the user in the Realm).

In this mode, all the functionality of the Vesting Only mode is available (with additional
accounts indicated in the Deposit/Withdraw/ChangeOwner instructions). The following
instructions have also been added:
 - `CreateVoterWeightRecord` - to create a record of a user's vote (with zero weight).
Used to prepare the vesting recipient for changeOwner.
 - `SetVotePercent` - allows the recipient of a vesting to specify what percentage of his
vesting to use for defining the weight of a vote (temporary implementation of the ability
to vote in parts, which will be implemented later in spl-governance).
This instruction can also be used by the delegate of a vesting recipient.

When withdrawing tokens, it is checked that the moment for their release has come,
as well as the absence of active proposals and votes from the recipient of a vesting.
