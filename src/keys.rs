// Copyright 2017-2018 int08h LLC
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

//!
//! Representations of Roughtime's online and long-term Ed25519 keys
//!

use message::RtMessage;
use sign::Signer;
use tag::Tag;
use time::Timespec;

use byteorder::{LittleEndian, WriteBytesExt};

use super::{CERTIFICATE_CONTEXT, SIGNED_RESPONSE_CONTEXT};
use std::fmt;
use std::fmt::Formatter;

///
/// Represents the delegated Roughtime ephemeral online key.
///
pub struct OnlineKey {
    signer: Signer,
}

impl OnlineKey {
    pub fn new() -> Self {
        OnlineKey {
            signer: Signer::new(),
        }
    }

    /// Create a DELE message containing the public key of this online key
    pub fn make_dele(&self) -> RtMessage {
        let zeros = [0u8; 8];
        let max = [0xff; 8];
        let pub_key_bytes = self.signer.public_key_bytes();

        let mut dele_msg = RtMessage::new(3);
        dele_msg.add_field(Tag::PUBK, pub_key_bytes).unwrap();
        dele_msg.add_field(Tag::MINT, &zeros).unwrap();
        dele_msg.add_field(Tag::MAXT, &max).unwrap();

        dele_msg
    }

    /// Create an SREP response containing the provided time and Merkle root,
    /// signed by this online key.
    pub fn make_srep(&mut self, now: Timespec, merkle_root: &[u8]) -> RtMessage {
        let mut radi = [0; 4];
        let mut midp = [0; 8];

        // one second (in microseconds)
        (&mut radi as &mut [u8])
            .write_u32::<LittleEndian>(1_000_000)
            .unwrap();

        // current epoch time in microseconds
        let midp_time = {
            let secs = (now.sec as u64) * 1_000_000;
            let nsecs = (now.nsec as u64) / 1_000;

            secs + nsecs
        };
        (&mut midp as &mut [u8])
            .write_u64::<LittleEndian>(midp_time)
            .unwrap();

        // Signed response SREP
        let srep_bytes = {
            let mut srep_msg = RtMessage::new(3);
            srep_msg.add_field(Tag::RADI, &radi).unwrap();
            srep_msg.add_field(Tag::MIDP, &midp).unwrap();
            srep_msg.add_field(Tag::ROOT, merkle_root).unwrap();

            srep_msg.encode().unwrap()
        };

        // signature on SREP
        let srep_signature = {
            self.signer.update(SIGNED_RESPONSE_CONTEXT.as_bytes());
            self.signer.update(&srep_bytes);
            self.signer.sign()
        };

        let mut result = RtMessage::new(2);
        result.add_field(Tag::SIG, &srep_signature).unwrap();
        result.add_field(Tag::SREP, &srep_bytes).unwrap();

        result
    }
}

impl fmt::Display for OnlineKey {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.signer)
    }
}

///
/// Represents the server's long-term identity.
///
pub struct LongTermKey {
    signer: Signer,
}

impl LongTermKey {
    pub fn new(seed: &[u8]) -> Self {
        LongTermKey {
            signer: Signer::from_seed(seed),
        }
    }

    /// Create a CERT message with a DELE containing the provided online key
    /// and a SIG of the DELE value signed by the long-term key
    pub fn make_cert(&mut self, online_key: &OnlineKey) -> RtMessage {
        let dele_bytes = online_key.make_dele().encode().unwrap();

        self.signer.update(CERTIFICATE_CONTEXT.as_bytes());
        self.signer.update(&dele_bytes);

        let dele_signature = self.signer.sign();

        let mut cert_msg = RtMessage::new(2);
        cert_msg.add_field(Tag::SIG, &dele_signature).unwrap();
        cert_msg.add_field(Tag::DELE, &dele_bytes).unwrap();

        cert_msg
    }
}

impl fmt::Display for LongTermKey {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.signer)
    }
}
