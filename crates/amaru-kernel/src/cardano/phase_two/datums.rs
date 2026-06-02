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

use crate::{Hash, MemoizedPlutusData, NonEmptyVec, PlutusData, PlutusDataSet, size::DATUM};

/// The datums supplied as witnesses in a transaction, keyed by hash.
///
/// A lookup table from a datum [`struct@Hash`] to the [`PlutusData`] it commits to. This is what
/// resolves a hash-only datum ([`MemoizedDatum::Hash`](crate::MemoizedDatum)) on a spent output back to the actual datum value;
/// inline datums carry their value already and need no entry here.
#[derive(Debug, Default)]
pub struct Datums<'a>(pub BTreeMap<Hash<DATUM>, &'a PlutusData>);

impl<'a> From<&'a NonEmptyVec<MemoizedPlutusData>> for Datums<'a> {
    fn from(plutus_data: &'a NonEmptyVec<MemoizedPlutusData>) -> Self {
        Self(plutus_data.iter().map(|data| (data.hash(), data.as_ref())).collect())
    }
}

impl<'a> From<&'a PlutusDataSet> for Datums<'a> {
    fn from(plutus_data: &'a PlutusDataSet) -> Self {
        Self::from(&**plutus_data)
    }
}
