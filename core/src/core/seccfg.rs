/*
    SPDX-License-Identifier: GPL-3.0-or-later
    SPDX-FileCopyrightText: 2025 Shomy

    Derived from:
    https://github.com/bkerler/mtkclient/blob/main/mtkclient/Library/Hardware/seccfg.py
    Original SPDX-License-Identifier: GPL-3.0-or-later
    Original SPDX-FileCopyrightText: 2018–2024 bkerler

    This file remains under the GPL-3.0-or-later license.
    However, as part of a larger project licensed under the AGPL-3.0-or-later,
    the combined work is subject to the networking terms of the AGPL-3.0-or-later,
    as for term 13 of the GPL-3.0-or-later license.
*/
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

const V4_MAGIC_BEGIN: u32 = 0x4D4D4D4D;
const V4_MAGIC_END: u32 = 0x45454545;

pub enum LockFlag {
    Lock,
    Unlock,
}

#[derive(Clone)]
pub enum SecCfgV4Algo {
    SW,
    HW,
    HWv3,
    HWv4,
}

#[derive(Default)]
pub struct SecCfgV4 {
    pub seccfg_ver: u32,
    pub seccfg_size: u32,
    pub lock_state: u32,
    pub critical_lock_state: u32,
    pub sboot_runtime: u32,
    algo: Option<SecCfgV4Algo>,
    enc_hash: Option<Vec<u8>>,
}

impl SecCfgV4 {
    pub fn new() -> Self {
        SecCfgV4 {
            seccfg_ver: 4,
            seccfg_size: 20,
            lock_state: 0,
            critical_lock_state: 0,
            sboot_runtime: 0,
            algo: None,
            enc_hash: None,
        }
    }

    pub fn parse_header(data: &[u8]) -> Result<SecCfgV4> {
        if data.len() < 0x20 {
            return Err(Error::penumbra("SecCfg v4 data too short"));
        }

        let magic = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let seccfg_ver = u32::from_le_bytes(data[4..8].try_into().unwrap());
        let seccfg_size = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let lock_state = u32::from_le_bytes(data[12..16].try_into().unwrap());
        let critical_lock_state = u32::from_le_bytes(data[16..20].try_into().unwrap());
        let sboot_runtime = u32::from_le_bytes(data[20..24].try_into().unwrap());
        let endflag = u32::from_le_bytes(data[24..28].try_into().unwrap());
        let enc_hash = data[28..60].to_vec();

        if magic != V4_MAGIC_BEGIN || endflag != V4_MAGIC_END {
            return Err(Error::penumbra("Invalid SecCfg v4 magic values"));
        }

        Ok(SecCfgV4 {
            seccfg_ver,
            seccfg_size,
            lock_state,
            critical_lock_state,
            sboot_runtime,
            algo: None,
            enc_hash: Some(enc_hash),
        })
    }

    pub fn get_hash(&self) -> Vec<u8> {
        let header_data = [
            V4_MAGIC_BEGIN.to_le_bytes(),
            self.seccfg_ver.to_le_bytes(),
            self.seccfg_size.to_le_bytes(),
            self.lock_state.to_le_bytes(),
            self.critical_lock_state.to_le_bytes(),
            self.sboot_runtime.to_le_bytes(),
            V4_MAGIC_END.to_le_bytes(),
        ]
        .concat();

        let hash = Sha256::digest(&header_data);
        hash.to_vec()
    }

    pub fn get_algo(&self) -> Option<SecCfgV4Algo> {
        self.algo.clone()
    }

    pub fn set_algo(&mut self, algo: SecCfgV4Algo) {
        self.algo = Some(algo);
    }

    pub fn set_encrypted_hash(&mut self, enc_hash: Vec<u8>) {
        self.enc_hash = Some(enc_hash);
    }

    pub fn get_encrypted_hash(&self) -> Vec<u8> {
        self.enc_hash.clone().unwrap_or_default()
    }

    pub fn set_lock_state(&mut self, lock_flag: LockFlag) {
        match lock_flag {
            LockFlag::Lock => {
                self.lock_state = 1;
                self.critical_lock_state = 1;
            }
            LockFlag::Unlock => {
                self.lock_state = 3;
                self.critical_lock_state = 0;
            }
        }
    }

    pub fn create(&mut self) -> Vec<u8> {
        let mut seccfg_data = Vec::new();
        seccfg_data.extend(&V4_MAGIC_BEGIN.to_le_bytes());
        seccfg_data.extend(&self.seccfg_ver.to_le_bytes());
        seccfg_data.extend(&self.seccfg_size.to_le_bytes());
        seccfg_data.extend(&self.lock_state.to_le_bytes());
        seccfg_data.extend(&self.critical_lock_state.to_le_bytes());
        seccfg_data.extend(&self.sboot_runtime.to_le_bytes());
        seccfg_data.extend(&V4_MAGIC_END.to_le_bytes());

        if let Some(enc_hash) = &self.enc_hash {
            seccfg_data.extend_from_slice(enc_hash);
        } else {
            let hash = self.get_hash();
            seccfg_data.extend_from_slice(&hash);
        }

        while !seccfg_data.len().is_multiple_of(0x200) {
            seccfg_data.push(0);
        }

        seccfg_data
    }
}
