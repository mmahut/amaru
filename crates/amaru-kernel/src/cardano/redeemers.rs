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

use std::{collections::BTreeMap, ops::Deref};

pub use pallas_primitives::conway::Redeemers as PallasRedeemers;

use crate::{BorrowedScript, Bytes, ExUnits, PlutusData, RedeemerKey, ScriptPurpose, cbor, empty_bytes};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Redeemers {
    #[serde(skip, default = "empty_bytes")]
    original_bytes: Bytes,
    inner: PallasRedeemers,
}

impl<C> cbor::Encode<C> for Redeemers {
    fn encode<W: cbor::encode::Write>(
        &self,
        e: &mut cbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), cbor::encode::Error<W::Error>> {
        self.inner.encode(e, ctx)
    }
}

impl<'b, C> cbor::Decode<'b, C> for Redeemers {
    fn decode(d: &mut cbor::Decoder<'b>, ctx: &mut C) -> Result<Self, cbor::decode::Error> {
        let (inner, bytes) = cbor::tee(d, |d| PallasRedeemers::decode(d, ctx))?;
        Ok(Self { original_bytes: Bytes::from(bytes.to_vec()), inner })
    }
}

impl Redeemers {
    pub fn original_bytes(&self) -> &[u8] {
        &self.original_bytes
    }
}

impl AsRef<PallasRedeemers> for Redeemers {
    fn as_ref(&self) -> &PallasRedeemers {
        &self.inner
    }
}

/// A redeemer resolved against the transaction and UTxO set.
///
/// An on-chain redeemer carries only a pointer ([`RedeemerKey`]: tag + index)
/// alongside its `data` and `ex_units`. Resolving that pointer yields the concrete
/// [`ScriptPurpose`] it acts on and the [`BorrowedScript`] it dispatches to, both captured
/// in a `RedeemerEntry`. Each entry is therefore the unit of a single Plutus script execution.
#[derive(Debug)]
pub struct RedeemerEntry<'a> {
    pub purpose: ScriptPurpose<'a>,
    pub data: &'a PlutusData,
    pub ex_units: ExUnits,
    pub script: BorrowedScript<'a>,
}

/// A transaction's redeemers, each resolved and indexed by its pointer.
///
/// Maps every [`RedeemerKey`] (tag + index) to its [`RedeemerEntry`].
/// Unlike [`PallasRedeemers`], which may arrive as either a list or a map,
/// this is always a deduplicated map: duplicate keys collapse last-wins, matching the ledger's `Map.fromList` semantics.
#[derive(Debug)]
pub struct PlutusRedeemers<'a>(BTreeMap<RedeemerKey, RedeemerEntry<'a>>);

impl<'a> PlutusRedeemers<'a> {
    pub fn new(inner: BTreeMap<RedeemerKey, RedeemerEntry<'a>>) -> Self {
        Self(inner)
    }
}

impl<'a> Deref for PlutusRedeemers<'a> {
    type Target = BTreeMap<RedeemerKey, RedeemerEntry<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PlutusRedeemers<'_> {
    pub fn iter_from<'a>(
        redeemers: &'a PallasRedeemers,
    ) -> Box<dyn Iterator<Item = (RedeemerKey, &'a PlutusData, ExUnits)> + 'a> {
        match redeemers {
            PallasRedeemers::List(list) => {
                Box::new(list.iter().map(|r| (RedeemerKey { tag: r.tag, index: r.index }, &r.data, r.ex_units)))
            }
            PallasRedeemers::Map(map) => {
                Box::new(map.iter().map(|(key, value)| (key.clone(), &value.data, value.ex_units)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MaybeIndefArray, Redeemer, RedeemerTag};

    #[test]
    fn iter_from_into_btreemap_keeps_last_for_duplicate_redeemers() {
        // Pins the property the bug fix is for: when two redeemers share (tag, index) but
        // carry different data/ex_units, the last occurrence wins, matching Haskell's
        // Map.fromList. The property holds because RedeemerKey is primitive: BTreeMap::insert
        // on Ord-equal keys replaces the value, and the value carries everything that varies.
        let make_redeemer = |mem: u64, steps: u64, payload: u8| Redeemer {
            tag: RedeemerTag::Spend,
            index: 0,
            data: PlutusData::BoundedBytes(vec![payload].into()),
            ex_units: ExUnits { mem, steps },
        };

        let r1 = make_redeemer(100, 200, 0xAA);
        let r2 = make_redeemer(999, 888, 0xBB);

        let pallas_redeemers = PallasRedeemers::List(MaybeIndefArray::Indef(vec![r1, r2.clone()]));

        let map: BTreeMap<RedeemerKey, (&PlutusData, ExUnits)> =
            PlutusRedeemers::iter_from(&pallas_redeemers).map(|(k, data, ex_units)| (k, (data, ex_units))).collect();

        assert_eq!(map.len(), 1, "duplicate (tag, index) should collapse to one entry");

        let (key, (data, ex_units)) = map.iter().next().unwrap();
        assert_eq!(key.tag, RedeemerTag::Spend);
        assert_eq!(key.index, 0);
        assert_eq!(*ex_units, r2.ex_units, "last redeemer's ex_units must win");
        assert_eq!(**data, r2.data, "last redeemer's data must win");
    }
}
