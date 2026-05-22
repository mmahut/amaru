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

use std::mem;

use amaru_kernel::{Epoch, ProtocolParameters, ProtocolVersion};

use crate::epoch_transition::{
    Computed, Effective, GovernanceActivity, GovernanceUpdates, PoolsEpochTransitionUpdates, Rewards, RewardsState,
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

    pub fn rewards(&self) -> &RewardsState {
        &self.rewards
    }

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
