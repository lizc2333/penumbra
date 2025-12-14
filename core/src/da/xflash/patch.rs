/*
    SPDX-License-Identifier: AGPL-3.0-or-later
    SPDX-FileCopyrightText: 2025 Shomy
*/

const EXT_LOADER: &[u8] = include_bytes!("../../../payloads/extloader_v5.bin");

use log::info;
use sha2::{Digest, Sha256};

use crate::da::xflash::XFlash;
use crate::da::{DA, DAEntryRegion};
use crate::error::Result;
use crate::utilities::arm::*;
use crate::utilities::patching::*;

/// Patches both DA1 and DA2, specific for V5 DA
pub fn patch_da(xflash: &mut XFlash) -> Result<DA> {
    let da2 = patch_da2(xflash)?;
    let mut da1 = patch_da1(xflash)?;

    let hash_pos = xflash.da.find_da_hash_offset();
    match hash_pos {
        Some(pos) => {
            let mut hasher = Sha256::new();
            hasher.update(&da2.data[..da2.data.len().saturating_sub(da2.sig_len as usize)]);
            let hash_result = hasher.finalize();
            patch(&mut da1.data, pos, &bytes_to_hex(&hash_result))?;

            let original_da = &xflash.da;
            let da = DA {
                da_type: xflash.da.da_type.clone(),
                regions: vec![original_da.regions[0].clone(), da1.clone(), da2.clone()],
                magic: original_da.magic,
                hw_code: original_da.hw_code,
                hw_sub_code: original_da.hw_sub_code,
            };
            Ok(da)
        }
        None => {
            info!("[Penumbra] Could not find DA1 hash position, skipping patching");
            Ok(xflash.da.clone())
        }
    }
}

/// Patches only DA1, specific for V5 DA
pub fn patch_da1(xflash: &mut XFlash) -> Result<DAEntryRegion> {
    let da1 = xflash.da.get_da1().cloned().unwrap();
    Ok(da1)
}

/// Patches only DA2, specific for V5 DA
pub fn patch_da2(xflash: &mut XFlash) -> Result<DAEntryRegion> {
    let mut da2 = xflash.da.get_da2().cloned().unwrap();

    patch_boot_to(&mut da2)?;

    Ok(da2)
}

/// Adds back the boot_to command to da2, allowing to load extensions.
/// This is needed only on DAs which build date is >= late 2023
fn patch_boot_to(da: &mut DAEntryRegion) -> Result<bool> {
    // We only need to patch if the DA doesn't support this cmd.
    if find_pattern(&da.data, "636D645F626F6F745F746F00", 0) != HEX_NOT_FOUND {
        return Ok(false);
    }

    let dagent_reg_cmds = find_pattern(&da.data, "08B54FF460200021XXF7", 0);
    let devc_read_reg = find_pattern(&da.data, "30B5002385B004460193", 0);
    let unsupported_cmd = find_pattern(&da.data, "084B13B504460193", 0);
    let register_maj_cmd = find_pattern(&da.data, "38B5054610200C46", 0);

    // Patch the devc_read_reg to be our new cmd
    patch(&mut da.data, devc_read_reg, &bytes_to_hex(EXT_LOADER))?;

    // Find the LDR of unsupported cmd and patch it with devc_read_reg address (thumb addr)
    let unsupported_cmd_addr = to_thumb_addr(unsupported_cmd, da.addr).to_le_bytes();
    let devc_read_reg_addr = to_thumb_addr(devc_read_reg, da.addr).to_le_bytes();

    // Patch the DAT to point to the new injected cmd
    let unsupported_cmd_dat = find_pattern(&da.data, &bytes_to_hex(&unsupported_cmd_addr), 0);
    patch(&mut da.data, unsupported_cmd_dat, &bytes_to_hex(&devc_read_reg_addr))?;

    let mut reg_cmd_patch = Vec::with_capacity(20);

    #[rustfmt::skip]
    reg_cmd_patch.extend_from_slice(&[
        0x4F, 0xF4, 0x80, 0x30, // mov.w r0, #0x10000
        0x08, 0x30,             // adds r0, #0x8
    ]);

    let ldr_off = dagent_reg_cmds + 0x2 + reg_cmd_patch.len();
    let ldr = encode_ldr(1, ldr_off, unsupported_cmd_dat, da.addr)?;

    // da_agent addr + skip push + length of the patch
    let bl_addr = ldr_off as u32 + 0x2 + da.addr;
    let reg_maj_cmd_addr = to_thumb_addr(register_maj_cmd, da.addr);
    let shellcode = encode_bl(bl_addr, reg_maj_cmd_addr);

    reg_cmd_patch.extend_from_slice(&ldr);
    reg_cmd_patch.extend_from_slice(&shellcode);
    reg_cmd_patch.extend_from_slice(&[0xAF, 0xF3, 0x00, 0x80]); // nop.w
    reg_cmd_patch.extend_from_slice(&[0xAF, 0xF3, 0x00, 0x80]); // nop.w

    patch(&mut da.data, dagent_reg_cmds + 0x2, &bytes_to_hex(&reg_cmd_patch))?;

    info!("[Penumbra] Patched DA2 to add cmd_boot_to");

    Ok(true)
}
