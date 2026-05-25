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

use amaru_kernel::{Epoch, EraHistory};
use amaru_observability::info_span;

use crate::{
    governance::ratification::RatificationContext,
    state::StateError,
    store::{ReadStore, StoreError},
};

mod pools_updates;
pub use pools_updates::PoolsEpochTransitionUpdates;

mod rewards_state;
pub use rewards_state::{Computed, Effective, Rewards, RewardsState};

mod ratification;
pub use ratification::{GovernanceActivity, GovernanceUpdates};

/// Ends the ongoing epoch by calculating rewards payouts to the various still-registered accounts.
/// Unpaid rewards are assigned back to the treasury.
///
pub fn end_epoch(db: &impl ReadStore, computed_rewards: Rewards<Computed>) -> Result<Rewards<Effective>, StoreError> {
    info_span!(amaru_observability::amaru::ledger::epoch_transition::END_EPOCH).in_scope(|| {
        // FIXME: account de-registrations from the volatile db
        //
        // The following code only looks at accounts from the stable store which is missing the last
        // `k` blocks of an epoch. One may unregister its account in that last unstable chunk; so
        // we must filter them out.
        let accounts = db.iter_accounts()?.map(|(k, _v)| k);
        Ok(Rewards::<Effective>::new(computed_rewards, accounts))
    })
}

pub fn begin_epoch<'distr>(
    db: &impl ReadStore,
    epoch: Epoch,
    era_history: &EraHistory,
    ratification_context: RatificationContext<'distr>,
) -> Result<(PoolsEpochTransitionUpdates, GovernanceUpdates), StateError> {
    info_span!(amaru_observability::amaru::ledger::epoch_transition::BEGIN_EPOCH).in_scope(|| {
        // FIXME: unbind accounts of unregistered pools
        //
        // We also need a mechanism to remove any remaining delegation to pools retired by this
        // step. The accounts are already filtered out when computing rewards, but if any retired
        // pool were to re-register, they would automatically be granted the stake associated to
        // their past delegates.

        // Tick pools to compute their new state at the epoch boundary. Notice
        // how we tick with the _current epoch_ however, but we take the snapshot before
        // the tick since the actions are only effective once the epoch is crossed.
        let pools_updates = PoolsEpochTransitionUpdates::new(db, epoch)?;

        // Ratify and enact proposals at the epoch boundary. Note that this does not modify the
        // immutable store in any fashion (db is read-only here) but produces a series of
        // governance updates to be applied to the database once stable; and use in-memory in the
        // meantime.
        let governance_updates = GovernanceUpdates::new(db, era_history, ratification_context)?;

        Ok((pools_updates, governance_updates))
    })
}
