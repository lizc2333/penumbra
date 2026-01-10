#!/usr/bin/env python3
#
# SPDX-FileCopyrightText: 2025 Shomy
# SPDX-License-Identifier: AGPL-3.0-or-later
#
from typing import Set

from parse_da import DA, DAEntryRegion, DAFile, DAType


def parse_hwcodes(arg: str) -> Set[int]:
    hwcodes: Set[int] = set()
    for part in arg.split(","):
        part = part.strip().lower()
        if part.startswith("0x"):
            hwcodes.add((int(part, 16)))
        else:
            hwcodes.add((int(part, 10)))

    return hwcodes


if __name__ == "__main__":
    import sys

    if len(sys.argv) not in (2, 3):
        print(f"Usage: {sys.argv[0]} <da_file>")
        sys.exit(1)

    da_path = sys.argv[1]
    wanted_hwcodes = None

    if len(sys.argv) == 3:
        wanted_hwcodes = parse_hwcodes(sys.argv[2])

    with open(sys.argv[1], "rb") as f:
        da_raw_data = f.read()

    da_file = DAFile.parse_da(da_raw_data)

    for da in da_file.das:
        hw_code = da.hw_code

        if wanted_hwcodes is not None and hw_code not in wanted_hwcodes:
            continue

        da1 = da.get_da1()
        da2 = da.get_da2()
        hw_code = hex(hw_code)

        da1_data = da1.data[: -da1.sig_len] if da1.sig_len > 0 else da1.data
        da2_data = da2.data[: -da2.sig_len] if da2.sig_len > 0 else da2.data

        with open(f"da1_{hw_code}.bin", "wb") as f:
            f.write(da1_data)
            print(f"Wrote da1.bin, size: {len(da1_data)} bytes")

        with open(f"da2_{hw_code}.bin", "wb") as f:
            f.write(da2_data)
            print(f"Wrote da2.bin, size: {len(da2_data)} bytes")

        if da1.sig_len > 0:
            with open(f"da1_{hw_code}.sig", "wb") as f:
                f.write(da1.data[-da1.sig_len :])
                print(f"Wrote da1.sig, size: {da1.sig_len} bytes")

        if da2.sig_len > 0:
            with open(f"da2_{hw_code}.sig", "wb") as f:
                f.write(da2.data[-da2.sig_len :])
                print(f"Wrote da2.sig, size: {da2.sig_len} bytes")

        print(f"Extracted DA for hw_code: {hw_code}")

    print("DA stages extracted successfully.")
