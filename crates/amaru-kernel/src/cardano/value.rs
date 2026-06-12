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

use std::{borrow::Cow, collections::BTreeMap};

pub use pallas_primitives::conway::Value;

use crate::{AssetName, NonEmptyKeyValuePairs, NonZeroInt};
pub use crate::{Hash, size::CREDENTIAL};

/// An identifier for a currency in a [`Value`].
///
///
/// This identifier is specifically used to enforce canonical ordering in a PlutusData representation of [`Value`].
/// Lovelace is encoded as the empty bytestring and, always sorts ahead of native assets.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CurrencySymbol {
    Lovelace,
    Native(Hash<CREDENTIAL>),
}

/// The assets minted and burned by a transaction.
///
/// A map from minting-policy [`struct@Hash`] to that policy's assets, each carrying a signed
/// quantity: positive mints, negative burns. Unlike [`Value`], amounts are signed and
/// there is no ada entry; only native assets can be minted or burned.
#[derive(Debug, Default)]
pub struct PlutusMint<'a>(pub BTreeMap<Hash<CREDENTIAL>, BTreeMap<Cow<'a, AssetName>, i64>>);

impl<'a> From<&'a NonEmptyKeyValuePairs<Hash<CREDENTIAL>, NonEmptyKeyValuePairs<AssetName, NonZeroInt>>>
    for PlutusMint<'a>
{
    fn from(value: &'a NonEmptyKeyValuePairs<Hash<CREDENTIAL>, NonEmptyKeyValuePairs<AssetName, NonZeroInt>>) -> Self {
        let mints = value
            .iter()
            .map(|(policy, multiasset)| {
                (
                    *policy,
                    multiasset
                        .iter()
                        .map(|(asset_name, amount)| (Cow::Borrowed(asset_name), (*amount).into()))
                        .collect(),
                )
            })
            .collect();

        Self(mints)
    }
}
