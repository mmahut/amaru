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

use crate::{Hash, MemoizedDatum, PlutusData, size::DATUM};

/// How a transaction output carries its datum.
///
/// An output may attach nothing, only the [`struct@Hash`] of a datum (whose value is then
/// supplied separately in the transaction's [`Datums`](super::datums::Datums)), or an `Inline` datum embedded
/// directly in the output. Borrowed from the underlying [`MemoizedDatum`].
#[derive(Debug, Clone)]
pub enum DatumOption<'a> {
    None,
    Hash(&'a Hash<DATUM>),
    Inline(&'a PlutusData),
}

impl<'a> From<&'a MemoizedDatum> for DatumOption<'a> {
    fn from(value: &'a MemoizedDatum) -> Self {
        match value {
            MemoizedDatum::None => Self::None,
            MemoizedDatum::Hash(hash) => Self::Hash(hash),
            MemoizedDatum::Inline(data) => Self::Inline(data.as_ref()),
        }
    }
}

impl<'a> From<Option<&'a Hash<DATUM>>> for DatumOption<'a> {
    fn from(value: Option<&'a Hash<DATUM>>) -> Self {
        match value {
            Some(hash) => Self::Hash(hash),
            None => Self::None,
        }
    }
}
