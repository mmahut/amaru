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

use crate::{AssetName, Bytes, Hash, Lovelace, size::CREDENTIAL};

/// An identifier for a currency in a [`Value`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CurrencySymbol {
    Lovelace,
    Native(Hash<CREDENTIAL>),
}

impl From<Hash<CREDENTIAL>> for CurrencySymbol {
    fn from(value: Hash<CREDENTIAL>) -> Self {
        Self::Native(value)
    }
}

/// A representation of `Value` used in Plutus
///
/// The ledger's `Value` contains both a `Coin` and, optionally, a `Multiasset`.
/// In Plutus, this is simply a single map, with an empty bytestring representing lovelace
#[derive(Debug, Clone)]
pub struct Value<'a>(pub BTreeMap<CurrencySymbol, BTreeMap<Cow<'a, AssetName>, u64>>);

impl<'a> From<&'a crate::Value> for Value<'a> {
    fn from(value: &'a crate::Value) -> Self {
        let assets = match value {
            crate::Value::Coin(coin) => {
                BTreeMap::from([(CurrencySymbol::Lovelace, BTreeMap::from([(Cow::Owned(Bytes::from(vec![])), *coin)]))])
            }
            crate::Value::Multiasset(coin, multiasset) => {
                let mut map = BTreeMap::new();
                map.insert(CurrencySymbol::Lovelace, BTreeMap::from([(Cow::Owned(Bytes::from(vec![])), *coin)]));
                multiasset.iter().for_each(|(policy_id, asset_bundle)| {
                    map.insert(
                        CurrencySymbol::Native(*policy_id),
                        asset_bundle
                            .iter()
                            .map(|(asset_name, amount)| (Cow::Borrowed(asset_name), amount.into()))
                            .collect(),
                    );
                });

                map
            }
        };

        Self(assets)
    }
}

impl<'a> From<crate::Value> for Value<'a> {
    fn from(value: crate::Value) -> Self {
        let assets = match value {
            crate::Value::Coin(coin) => {
                BTreeMap::from([(CurrencySymbol::Lovelace, BTreeMap::from([(Cow::Owned(Bytes::from(vec![])), coin)]))])
            }
            crate::Value::Multiasset(coin, multiasset) => {
                let mut map = BTreeMap::new();
                map.insert(CurrencySymbol::Lovelace, BTreeMap::from([(Cow::Owned(Bytes::from(vec![])), coin)]));
                multiasset.into_iter().for_each(|(policy_id, asset_bundle)| {
                    map.insert(
                        CurrencySymbol::Native(policy_id),
                        asset_bundle
                            .into_iter()
                            .map(|(asset_name, amount)| (Cow::Owned(asset_name), amount.into()))
                            .collect(),
                    );
                });

                map
            }
        };

        Self(assets)
    }
}

impl From<Lovelace> for Value<'_> {
    fn from(coin: Lovelace) -> Self {
        Self(BTreeMap::from([(CurrencySymbol::Lovelace, BTreeMap::from([(Cow::Owned(Bytes::from(vec![])), coin)]))]))
    }
}

impl Value<'_> {
    pub fn ada(&self) -> Option<u64> {
        self.0.get(&CurrencySymbol::Lovelace).and_then(|asset_bundle| {
            asset_bundle.iter().find_map(
                |(name, amount)| {
                    if name.is_empty() && amount != &0 { Some(*amount) } else { None }
                },
            )
        })
    }
}
