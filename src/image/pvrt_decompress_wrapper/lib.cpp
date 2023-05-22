/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#include "../../../vendor/PVRTDecompress/PVRTDecompress.cpp"
extern "C" {
uint32_t touchHLE_decompress_pvrtc(const void *pvrtc_data, bool is_2bit,
                                   uint32_t width, uint32_t height,
                                   uint8_t *rgba_data) {
  return pvr::PVRTDecompressPVRTC(pvrtc_data, is_2bit, width, height,
                                  rgba_data);
}
}
