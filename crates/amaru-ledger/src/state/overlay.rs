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
    mem,
};

use amaru_kernel::{
    AsHash, ConstitutionalCommitteeStatus, Epoch, Lovelace, PoolId, ProtocolParameters, ProtocolVersion,
    RationalNumber, StakeCredential, StakeCredentialKind,
};
use amaru_observability::{info_span, trace_span};
use num::BigUint;
use tracing::debug;

use crate::{
    epoch_transition::{
        Computed, Effective, GovernanceActivity, GovernanceUpdates, PoolsEpochTransitionUpdates, Rewards, RewardsState,
    },
    governance::ratification::CommitteeUpdate,
    state::StateError,
    store::{EpochTransitionProgress, Store, StoreError, TransactionalContext, columns::pools::Row as Pool},
};

/// Represents the information we sometimes have to overlay on top of the immutable store. That is,
/// they are computed bits of the ledger state that aren't stable yet but that still need to be
/// accounted for for block validation. They are computed at each epoch boundaries, and flushed
/// once we've reached enter the stability window of each epoch.
pub struct StateOverlay {
    /// The last known epoch; or said differently, the epoch for which this overlay is valid.
    epoch: Epoch,

    /// The computed rewards summary to be applied on the next epoch boundary. This is computed
    /// once in the epoch, and held until the end where it is reset.
    ///
    /// It also contains the latest stake distribution computed from the previous epoch, which we
    /// hold onto the epoch boundary. In the epoch boundary, the stake distribution becomes
    /// available for the leader schedule verification, whereas the stake distribution previously
    /// used for leader schedule is moved as rewards stake.
    rewards: RewardsState,

    /// Computed pools updates that are pending application to the stable store. The value is only
    /// `Some` during the first `k` blocks of an epoch since this corresponds to the unstable part
    /// of an epoch.
    ///
    /// When present, they must be taken into account when creating the ledger validation context.
    pools_updates: Option<PoolsEpochTransitionUpdates>,

    /// The result of an epoch boundary ratification, stashed temporarily until it is stable enough
    /// to persist in the stable storage.
    governance_updates: Option<GovernanceUpdates>,

    /// Updatable protocol parameters, cached from the stable store.
    protocol_parameters: ProtocolParameters,

    /// Track the number of dormant epochs (i.e. epochs that start without any available proposals).
    governance_activity: GovernanceActivity,
}

impl StateOverlay {
    /// Construct a new default/empty overlay from current parameters.
    pub fn new(epoch: Epoch, protocol_parameters: ProtocolParameters, governance_activity: GovernanceActivity) -> Self {
        Self {
            epoch,
            rewards: RewardsState::NotReady,
            pools_updates: None,
            governance_updates: None,
            protocol_parameters,
            governance_activity,
        }
    }
}

impl StateOverlay {
    /// Rollback an existing overlay, throwing away the epoch transition calculations.
    pub fn rollback(&mut self) {
        self.epoch = self.epoch - 1;
        self.rewards = match mem::take(&mut self.rewards) {
            st @ RewardsState::NotReady | st @ RewardsState::Computed(..) => st,
            RewardsState::Effective(effective) => RewardsState::Computed(effective.into()),
        };
        self.pools_updates = None;
        self.governance_updates = None;
    }

    /// Record transition into a new epoch.
    pub fn transition(
        &mut self,
        effective_rewards: Rewards<Effective>,
        pools_updates: PoolsEpochTransitionUpdates,
        governance_updates: GovernanceUpdates,
    ) {
        self.epoch = self.epoch + 1;
        self.rewards = RewardsState::Effective(effective_rewards);
        self.pools_updates = Some(pools_updates);
        self.governance_updates = Some(governance_updates);
    }

    /// Flush an overlay to disk.
    pub fn apply(&mut self, db: &impl Store) -> Result<(), StateError> {
        info_span!(amaru_observability::amaru::ledger::state::APPLYING_OVERLAY, epoch = u64::from(self.epoch)).in_scope(
            || {
                // ---------------------------------------------------------------------------- End of epoch
                db.with_transaction::<_, StateError>(|batch| {
                    let should_end_epoch =
                        batch.try_epoch_transition(None, Some(EpochTransitionProgress::EpochEnded))?;

                    if should_end_epoch {
                        if let RewardsState::Effective(effective_rewards) = mem::take(&mut self.rewards) {
                            pay_rewards(batch, effective_rewards)?;
                        } else {
                            return Err(StateError::NoEffectiveRewards);
                        }
                    }

                    Ok(())
                })?;

                // ------------------------------------------------------------------------------ Snapshot
                db.with_transaction::<_, StateError>(|batch| {
                    let should_snapshot = batch.try_epoch_transition(
                        Some(EpochTransitionProgress::EpochEnded),
                        Some(EpochTransitionProgress::SnapshotTaken),
                    )?;

                    if should_snapshot {
                        db.next_snapshot(self.epoch - 1)?;
                    }

                    Ok(())
                })?;

                // -------------------------------------------------------------------------- Start of epoch
                db.with_transaction::<_, StateError>(|batch| {
                    let should_begin_epoch = batch.try_epoch_transition(
                        Some(EpochTransitionProgress::SnapshotTaken),
                        Some(EpochTransitionProgress::EpochStarted),
                    )?;

                    if should_begin_epoch {
                        reset_blocks_count(batch)?;

                        reset_fees(batch)?;

                        if let Some(mut pools_updates) = mem::take(&mut self.pools_updates) {
                            update_pools(batch, pools_updates.take_updated(), pools_updates.take_retired())?;
                            pay_accounts(batch, pools_updates.refunds())?;
                        }

                        if let Some(governance_updates) = mem::take(&mut self.governance_updates) {
                            let (protocol_parameters, governance_activity) = enact_updates(batch, governance_updates)?;
                            self.protocol_parameters = protocol_parameters;
                            self.governance_activity = governance_activity;
                        }
                    }

                    Ok(())
                })
            },
        )
    }
}

impl StateOverlay {
    /// Check whether the overlay has unapplied state
    pub fn is_empty(&self) -> bool {
        matches!(&self.rewards, RewardsState::NotReady | RewardsState::Computed(..))
            && self.pools_updates.is_none()
            && self.governance_updates.is_none()
    }

    /// The last known epoch; or said differently, the epoch for which this overlay is valid.
    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    /// Get current protocol version, applying the overlay if necessary.
    pub fn protocol_version(&self) -> ProtocolVersion {
        let (major, minor) = self.protocol_parameters().protocol_version;
        (major, minor)
    }

    /// Obtain the protocol parameters for a specific epoch; which can either be the *current*
    /// epoch as per the latest tip; or the previous one. This is useful when applying the last
    /// `k` blocks of an epoch.
    ///
    /// At this point, the tip has already transitioned, but we still need some of the protocol
    /// parameters *at the time of that block* during persistence; mostly because of branching
    /// logic that depends on protocol version.
    pub fn protocol_parameters_for(&self, epoch: Epoch) -> &ProtocolParameters {
        if epoch == self.epoch {
            self.protocol_parameters()
        } else {
            self.assert_previous_epoch(epoch);
            &self.protocol_parameters
        }
    }

    /// Obtain the latest protocol parameters, from the overlay if any.
    pub fn protocol_parameters(&self) -> &ProtocolParameters {
        self.governance_updates.as_ref().map(|update| &update.protocol_parameters).unwrap_or(&self.protocol_parameters)
    }

    /// Similar to [`protocol_parameters_for`], we need to hold onto the governance activity at the
    /// time of a block, and not the value at the tip (since we apply block with ~2160 blocks of
    /// delays.
    pub fn governance_activity_for(&self, epoch: Epoch) -> GovernanceActivity {
        if epoch == self.epoch {
            self.governance_activity()
        } else {
            self.assert_previous_epoch(epoch);
            self.governance_activity
        }
    }

    /// Obtain the latest governance activity, from the overlay if any.
    pub fn governance_activity(&self) -> GovernanceActivity {
        let mut governance_activity = self.governance_activity;

        if self.governance_updates.as_ref().is_some_and(|updates| updates.is_dormant_epoch) {
            governance_activity.consecutive_dormant_epochs += 1;
        }

        governance_activity
    }

    /// Obtain a mutable reference to the governance activity, for updating after a block
    /// application.
    pub fn governance_activity_mut(&mut self) -> &mut GovernanceActivity {
        &mut self.governance_activity
    }

    /// A read-only handle on the rewards state.
    pub fn rewards(&self) -> &RewardsState {
        &self.rewards
    }

    /// A mut handle on the rewards state. Use with care to replace rewards.
    pub fn rewards_mut(&mut self) -> &mut RewardsState {
        &mut self.rewards
    }

    /// Consume a computed summary from a previous computation and mark the rewards as 'NotReady'.
    pub fn take_computed_rewards(&mut self) -> Option<Rewards<Computed>> {
        self.rewards.take_computed_rewards()
    }

    fn assert_previous_epoch(&self, epoch: Epoch) {
        assert!(
            epoch + 1 == self.epoch,
            "invariant violation: asking protocol parameters for an epoch that's neither current ({}) nor the precedent ({})",
            self.epoch,
            self.epoch.saturating_sub(1),
        );
    }
}

// Operations on the state
// ----------------------------------------------------------------------------

pub fn pay_rewards<'store>(
    db: &impl TransactionalContext<'store>,
    mut effective_rewards: Rewards<Effective>,
) -> Result<(), StoreError> {
    // Pay rewards out to every account
    db.with_accounts(|iterator| {
        for (account, mut row) in iterator {
            let rewards = effective_rewards.pop_account(&account);
            // The condition avoids the mutable borrow when not needed,
            // which will incur a db operation.
            if rewards > 0
                && let Some(account) = row.borrow_mut()
            {
                account.rewards += rewards;
            }
        }
    })?;

    // Technically, if we did everything *right*, there should be no accounts with rewards that
    // cannot be paid out (i.e. accounts that no longer exists). This has been taken care of during
    // the epoch transition calculations already. So at this point, this invariant must hold.
    assert!(
        effective_rewards.accounts().is_empty(),
        "unclaimed rewards when applying overlay: {:#?}",
        effective_rewards.accounts(),
    );

    // Adjust treasury and reserves accordingly.
    db.with_pots(|mut row| {
        let pots = row.borrow_mut();
        pots.treasury += effective_rewards.delta_treasury();
        pots.reserves -= effective_rewards.delta_reserves();
    })?;

    Ok(())
}

pub fn reset_fees<'store>(db: &impl TransactionalContext<'store>) -> Result<(), StoreError> {
    trace_span!(amaru_observability::amaru::ledger::state::RESET_FEES).in_scope(|| {
        db.with_pots(|mut row| {
            row.borrow_mut().fees = 0;
        })
    })
}

pub fn reset_blocks_count<'store>(db: &impl TransactionalContext<'store>) -> Result<(), StoreError> {
    trace_span!(amaru_observability::amaru::ledger::state::RESET_BLOCKS_COUNT).in_scope(|| {
        // TODO: Dropping entire RocksDB columns
        //
        // If necessary, come up with a more efficient way of dropping a "table".
        // RocksDB does support batch-removing of key ranges, but somehow, not in a
        // transactional way. So it isn't as trivial to implement as it may seem.
        db.with_block_issuers(|iterator| {
            for (_, mut row) in iterator {
                *row.borrow_mut() = None;
            }
        })
    })
}

pub fn update_pools<'store, 'iter>(
    db: &impl TransactionalContext<'store>,
    mut updates: BTreeMap<PoolId, Pool>,
    mut retirements: BTreeSet<PoolId>,
) -> Result<(), StoreError> {
    // TODO: multi-modify without full iterations?
    //
    // This quite inefficient, as we have to iterate through ALL pools just to possibly update a
    // few. It is reasonable to assume that the number of updates is vastly smaller to the total
    // number of pools. I don't feel like modifying the store handle to do that now, though...
    //
    // Given that the total number of pools is limited anyway; this is "acceptable".
    db.with_pools(|iterator| {
        for (id, mut row) in iterator {
            if retirements.remove(&id) {
                *row.borrow_mut() = None;
            } else if let Some(pool) = updates.remove(&id) {
                *row.borrow_mut() = Some(pool)
            }
        }
    })
}

/// Return deposits back to reward accounts, adding leftovers to the treasury.
pub fn pay_accounts<'store, 'iter>(
    db: &impl TransactionalContext<'store>,
    payouts: impl IntoIterator<Item = (&'iter StakeCredential, Lovelace)>,
) -> Result<(), StoreError> {
    let leftovers =
        payouts.into_iter().try_fold::<_, _, Result<_, StoreError>>(0, |leftovers, (account, deposit)| {
            debug!(
                type = %StakeCredentialKind::from(account),
                account = %account.as_hash(),
                %deposit,
                "payout"
            );

            Ok(leftovers + db.refund(account, deposit)?)
        })?;

    if leftovers > 0 {
        debug!(?leftovers, "payout");
        db.with_pots(|mut pots| pots.borrow_mut().treasury += leftovers)?;
    }

    Ok(())
}

pub fn enact_updates<'store, 'iter>(
    db: &impl TransactionalContext<'store>,
    mut updates: GovernanceUpdates,
) -> Result<(ProtocolParameters, GovernanceActivity), StoreError> {
    db.set_proposals_roots(&updates.roots)?;

    if let Some(new_constitution) = updates.new_constitution.take() {
        db.set_constitution(&new_constitution)?;
    }

    if let Some(committee_update) = updates.constitutional_committee.take() {
        update_constitutional_committee(db, committee_update)?;
    }

    db.remove_proposals(updates.pruned_proposals)?;

    let mut governance_activity = db.governance_activity()?;

    if updates.is_dormant_epoch {
        governance_activity.consecutive_dormant_epochs += 1;
        db.set_governance_activity(governance_activity)?;
    }

    db.set_protocol_parameters(&updates.protocol_parameters)?;

    pay_accounts(db, updates.payouts.iter().map(|(k, v)| (k, *v)))?;

    Ok((updates.protocol_parameters, governance_activity))
}

pub fn update_constitutional_committee<'store, 'iter>(
    db: &impl TransactionalContext<'store>,
    committee_update: CommitteeUpdate,
) -> Result<(), StoreError> {
    match committee_update {
        CommitteeUpdate::NoConfidence => {
            db.update_constitutional_committee(
                &ConstitutionalCommitteeStatus::NoConfidence,
                BTreeMap::new(),
                BTreeSet::new(),
            )?;

            db.with_cc_members(|iterator| {
                // NOTE: CC members are not deleted when entering no confidence
                // mode. They are simply marked as inactive.
                //
                // In particular, their hot<->cold bindings are preserved.
                for (_, mut row) in iterator {
                    if let Some(cc_member) = row.borrow_mut() {
                        cc_member.valid_until = None;
                    }
                }
            })
        }

        CommitteeUpdate::ChangeMembers { removed, added, threshold } => {
            let unsafe_u64 = |lbl: &str, n: &BigUint| {
                n.try_into().unwrap_or_else(|e| unreachable!("threshold {lbl}={n} larger than u64?!: {e}"))
            };

            let committee_status = ConstitutionalCommitteeStatus::Trusted {
                threshold: RationalNumber {
                    numerator: unsafe_u64("numerator", threshold.numer()),
                    denominator: unsafe_u64("denominator", threshold.denom()),
                },
            };

            db.update_constitutional_committee(&committee_status, added, removed)
        }
    }
}
