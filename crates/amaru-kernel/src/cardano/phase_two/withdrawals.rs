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

use thiserror::Error;

use super::stake_address::StakeAddress;
use crate::{Address, AddressError, Lovelace, NonEmptyKeyValuePairs as PallasNonEmptyKeyValuePairs, RewardAccount};

/// The reward withdrawals requested by a transaction.
///
/// A map from the [`StakeAddress`] being withdrawn from to the amount of [`Lovelace`]
/// taken. The [`StakeAddress`] key supplies the Plutus-canonical ordering, so this
/// `BTreeMap` iterates, and serializes, in the order a script expects; this is the type
/// that wrapper exists to serve.
#[derive(Debug, Default)]
pub struct Withdrawals(pub BTreeMap<StakeAddress, Lovelace>);

#[doc(hidden)]
#[derive(Debug, Error)]
pub enum WithdrawalError {
    #[error("invalid reward account: {0}")]
    InvalidRewardAccount(#[from] AddressError),
    #[error("invalid address type: {0}")]
    InvalidAddressType(Address),
}

impl TryFrom<&PallasNonEmptyKeyValuePairs<RewardAccount, Lovelace>> for Withdrawals {
    type Error = WithdrawalError;

    fn try_from(value: &PallasNonEmptyKeyValuePairs<RewardAccount, Lovelace>) -> Result<Self, Self::Error> {
        let withdrawals = value
            .iter()
            .map(|(reward_account, coin)| {
                let address = Address::from_bytes(reward_account)?;

                if let Address::Stake(reward_account) = address {
                    Ok((StakeAddress(reward_account), *coin))
                } else {
                    Err(WithdrawalError::InvalidAddressType(address))
                }
            })
            .collect::<Result<BTreeMap<_, _>, WithdrawalError>>()?;

        Ok(Self(withdrawals))
    }
}
