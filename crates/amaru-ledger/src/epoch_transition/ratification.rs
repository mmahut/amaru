// Copyright 2026 PRAGMA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use amaru_kernel::{
    ComparableProposalId,
    Constitution,
    Epoch,
    EraHistory,
    Lovelace,
    ProtocolParameters,
    StakeCredential,
    cbor,
    // NOTE: We have to import cbor as minicbor here because we derive 'Encode' and 'Decode' traits
    // instances for some types, and the macro rule handling that seems to be explicitly looking
    // for 'minicbor' in scope, and not an alias of any sort...
    cbor as minicbor,
    expect_stake_credential,
};
use amaru_observability::info_span;
use tracing::debug;

use crate::{
    governance::ratification::{
        CandidateProposal, CommitteeUpdate, ProposalsRoots, ProposalsRootsRc, RatificationContext,
    },
    state::StateError,
    store::columns::proposals::Row as Proposal,
};

/// A summary of the governance updates resulting from processing proposals at an epoch boundary.
/// The outcomes are initially stored in this object in-memory before being later flushed to the
/// stable store.
#[derive(Debug)]
pub struct GovernanceUpdates {
    /// Resulting proposal roots for each of the proposal categories.
    pub roots: ProposalsRoots,

    /// Resulting protocol parameters, includes protocol version upgrades for hard forks.
    pub protocol_parameters: ProtocolParameters,

    /// Proposals that have been ratified, have expired or have been pruned due to another
    /// conflicting proposal being dropped.
    pub pruned_proposals: BTreeSet<ComparableProposalId>,

    /// Payouts done to accounts; either because of a deposit refunds or because of a treasury
    /// withdrawal.
    pub payouts: BTreeMap<StakeCredential, Lovelace>,

    /// Captures whether the resulting epoch is considered 'dormant' (i.e. no active proposals
    /// left to vote on at the beginning of the epoch, after ratification).
    pub is_dormant_epoch: bool,

    /// The current constitutional committee, if any. No committee signals a state of
    /// no-confidence.
    pub constitutional_committee: Option<CommitteeUpdate>,

    /// A new constitution that has been voted and approved, if any.
    pub new_constitution: Option<Constitution>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, cbor::Encode, cbor::Decode, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceActivity {
    #[n(0)]
    pub consecutive_dormant_epochs: u32,
}

/// An intermediate type to capture meta-information related to a particular proposal. This is
/// information common to all proposals.
#[derive(Debug)]
struct ProposalMetadata {
    valid_until: Epoch,
    return_account: StakeCredential,
    deposit: Lovelace,
}

impl GovernanceUpdates {
    /// Look at every still-active governance proposal and ratify them in order of priority and
    /// submission.
    ///
    /// This generates a few outcomes:
    ///
    /// 1. There are some governance consequences such as withdrawals, protocol parameters changes,
    ///    etc...
    ///
    /// 2. Proposals that:
    ///
    ///     - are ratified
    ///     - are dropped due to other conflicting proposals
    ///     - expire
    ///
    ///    Will trigger a refund of their original deposit amount to the registered credential.
    ///    Their corresponding votes can also be pruned from the database.
    ///
    /// 3. The 'governance activity' must be updated accordingly; it captures the number of
    ///    consecutive dormant epochs, which is used to figure out DReps inactivity (DReps
    ///    aren't penalized for not being active in epochs where there's no activity).
    ///
    pub fn new(
        roots: ProposalsRootsRc,
        iter_proposals: impl Iterator<Item = (ComparableProposalId, Proposal)>,
        era_history: &EraHistory,
        mut ctx: RatificationContext<'_>,
    ) -> Result<Self, StateError> {
        let mut proposals_metadata: BTreeMap<Rc<ComparableProposalId>, ProposalMetadata> = BTreeMap::new();

        // A dual fold where we split the proposal information between 'CandidateProposal' and
        // 'ProposalMetadata'; both used in different contexts.
        let proposals: Vec<(Rc<ComparableProposalId>, CandidateProposal)> = iter_proposals
            .map(|(id, row)| {
                let id = Rc::new(id);

                let candidate = CandidateProposal {
                    valid_until: row.valid_until,
                    proposed_in: row.proposed_in,
                    governance_action: row.proposal.gov_action,
                };

                let metadata = ProposalMetadata {
                    valid_until: row.valid_until,
                    return_account: expect_stake_credential(&row.proposal.reward_account),
                    deposit: row.proposal.deposit,
                };

                proposals_metadata.insert(id.clone(), metadata);

                (id, candidate)
            })
            .collect();

        info_span!(
            amaru_observability::amaru::ledger::epoch_transition::GOVERNANCE_UPDATES_NEW,
            proposals_count = proposals.len() as u64
        )
        .in_scope(|| {
            let roots = ctx
                .ratify_proposals(
                    era_history,
                    // Get all proposals to ratify / enact. Note that, even though the ratification happens
                    // with an epoch of delay (and thus, using data from a snapshot), we always use the most
                    // recent set of proposals available. While recently submitted proposals won't have any
                    // votes, they might still end up being pruned due to a previous proposal being enacted.
                    //
                    // FIXME: Lazily fetch governance proposals on epoch boundary
                    //
                    // We shouldn't collect all proposals here, but provides iterators for the
                    // ratification step to go over them lazily.
                    proposals,
                    roots,
                )
                .map_err(|e| StateError::RatificationFailed(e.to_string()))?;

            // Once ratified, we can go over each proposal and figure out refunds due to
            // enactment, expiry or conflicts with other enacte proposals.
            let mut is_dormant_epoch = true;
            let mut payouts = ctx.withdrawals;
            for (id, proposal) in proposals_metadata.into_iter() {
                let expired = ctx.epoch == proposal.valid_until;
                let ratified_or_evicted = ctx.pruned_proposals.contains(&id);

                if expired || ratified_or_evicted {
                    debug!(name: "ratification.pruning", proposal_id = %id, expired, ratified_or_evicted);
                    payouts
                        .entry(proposal.return_account)
                        .and_modify(|balance| *balance += proposal.deposit)
                        .or_insert(proposal.deposit);
                } else {
                    // An epoch is said to be 'dormant' if there's no active proposals at the beginning of
                    // the epoch, after ratification has occured.
                    is_dormant_epoch = false;
                }
            }

            // NOTE: 'unwrap_or_clone' pruned proposal ids
            //
            // We have disposed of the proposals metadata just before by consuming the object via
            // 'into_iter'. This object should constitutes the last remaining Rc counts for the
            // proposal ids, so that the next 'unwrap_or_clone' should in practice results in a
            // clean transfer of ownership without clone.
            let pruned_proposals = ctx.pruned_proposals.into_iter().map(Rc::unwrap_or_clone).collect();

            Ok(Self {
                roots: roots.unwrap_or_clone(),
                pruned_proposals,
                payouts,
                is_dormant_epoch,
                protocol_parameters: ctx.protocol_parameters,
                new_constitution: ctx.new_constitution,
                constitutional_committee: ctx.constitutional_committee_update,
            })
        })
    }
}
