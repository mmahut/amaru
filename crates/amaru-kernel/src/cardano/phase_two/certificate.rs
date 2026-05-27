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

use std::ops::Deref;

use crate::{Certificate as PallasCertificate, ProtocolVersion};

/// A ledger certificate paired with the protocol version that governs its encoding.
///
/// Wraps a borrowed [`PallasCertificate`] and [`Deref`]s to it, so it behaves like the
/// underlying certificate everywhere except that it also carries a [`ProtocolVersion`].
/// That version is needed because a certificate's `ToPlutusData` encoding is
/// version-dependent: protocol 9 omits the deposit on the newer registration
/// certificates, a quirk that protocol 10 fixes.
#[derive(Debug, Clone)]
pub struct Certificate<'a> {
    pub protocol_version: ProtocolVersion,
    pub certificate: &'a PallasCertificate,
}

impl<'a> Deref for Certificate<'a> {
    type Target = PallasCertificate;

    fn deref(&self) -> &Self::Target {
        self.certificate
    }
}
