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

use std::collections::BTreeMap;

use crate::{ComparableProposalId, NonEmptyKeyValuePairs, ProposalId, Vote, Voter, VotingProcedure};

/// The governance votes cast by a transaction.
///
/// A nested map from [`Voter`] to the [`Vote`] (yes/no/abstain) it casts on each
/// governance action, keyed by [`ComparableProposalId`]. Only the decision is kept,
/// the on-chain [`VotingProcedure`]'s anchor is dropped, since scripts never see it.
#[derive(Debug, Default)]
pub struct Votes<'a>(pub BTreeMap<&'a Voter, BTreeMap<ComparableProposalId, &'a Vote>>);

impl<'a> From<&'a NonEmptyKeyValuePairs<Voter, NonEmptyKeyValuePairs<ProposalId, VotingProcedure>>> for Votes<'a> {
    fn from(
        voting_procedures: &'a NonEmptyKeyValuePairs<Voter, NonEmptyKeyValuePairs<ProposalId, VotingProcedure>>,
    ) -> Self {
        Self(
            voting_procedures
                .iter()
                .map(|(voter, votes)| {
                    (
                        voter,
                        votes
                            .iter()
                            .map(|(proposal, procedure)| {
                                (ComparableProposalId::from(proposal.clone()), &procedure.vote)
                            })
                            .collect(),
                    )
                })
                .collect(),
        )
    }
}
