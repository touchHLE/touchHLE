/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#define DR_MP3_IMPLEMENTATION
#define DR_MP3_NO_STDIO
#include "../../../vendor/dr_libs/dr_mp3.h"

#include <stdint.h>
#include <stdlib.h>

int16_t *touchHLE_decode_mp3_to_pcm(const uint8_t *data, size_t data_size,
                                    uint32_t *channels, uint32_t *sample_rate,
                                    uint64_t *frame_count) {
  drmp3_config config;
  int16_t *samples = drmp3_open_memory_and_read_pcm_frames_s16(
      data, data_size, &config, frame_count,
      /* pAllocationCallbacks: */ NULL);
  if (samples) {
    *channels = config.channels;
    *sample_rate = config.sampleRate;
  }
  return samples;
}

void touchHLE_free_decoded_mp3_pcm(int16_t *samples) {
  drmp3_free(samples, /* pAllocationCallbacks: */ NULL);
}
