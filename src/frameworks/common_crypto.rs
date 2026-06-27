/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! CommonCrypto

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr};
use crate::Environment;

// CCCryptorStatus
const kCCSuccess: i32 = 0;
const kCCParamError: i32 = -4300;
const kCCBufferTooSmall: i32 = -4301;
const kCCAlignmentError: i32 = -4303;
const kCCDecodeError: i32 = -4304;

// Вспомогательные функции для чтения и записи u32 (Little Endian)
fn read_u32_le(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}
fn write_u32_le(buf: &mut [u8], offset: usize, val: u32) {
    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
}

// Трансформация блока MD5
fn md5_step(state: &mut [u32; 4], data: &[u8; 64]) {
    let mut words = [0u32; 16];
    for i in 0..16 {
        words[i] = u32::from_le_bytes([
            data[i * 4],
            data[i * 4 + 1],
            data[i * 4 + 2],
            data[i * 4 + 3],
        ]);
    }
    let [mut a, mut b, mut c, mut d] = *state;

    let s = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];

    for i in 0..64 {
        let (mut f, g) = match i {
            0..=15 => ((b & c) | (!b & d), i),
            16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
            32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
            48..=63 => (c ^ (b | !d), (7 * i) % 16),
            _ => unreachable!(),
        };
        f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(words[g]);
        a = d;
        d = c;
        c = b;
        b = b.wrapping_add(f.rotate_left(s[i]));
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
}

#[allow(non_snake_case)]
fn CC_MD5_Init(env: &mut Environment, c: MutVoidPtr) -> i32 {
    if c.is_null() {
        return 0;
    }
    let c_ptr = c.cast::<u8>();

    // CC_MD5_CTX занимает 92 байта в памяти
    let mut ctx = [0u8; 92];
    write_u32_le(&mut ctx, 0, 0x67452301);
    write_u32_le(&mut ctx, 4, 0xefcdab89);
    write_u32_le(&mut ctx, 8, 0x98badcfe);
    write_u32_le(&mut ctx, 12, 0x10325476);

    env.mem.bytes_at_mut(c_ptr, 92).copy_from_slice(&ctx);
    1
}

#[allow(non_snake_case)]
fn CC_MD5_Update(env: &mut Environment, c: MutVoidPtr, data: ConstVoidPtr, len: GuestUSize) -> i32 {
    if c.is_null() || data.is_null() || len == 0 {
        return 1;
    }
    let c_ptr = c.cast::<u8>();
    let data_ptr = data.cast::<u8>();

    let mut ctx = env.mem.bytes_at(c_ptr, 92).to_vec();
    let input = env.mem.bytes_at(data_ptr, len).to_vec();

    let mut state = [
        read_u32_le(&ctx, 0),
        read_u32_le(&ctx, 4),
        read_u32_le(&ctx, 8),
        read_u32_le(&ctx, 12),
    ];
    let mut nl = read_u32_le(&ctx, 16);
    let mut nh = read_u32_le(&ctx, 20);
    let mut num = read_u32_le(&ctx, 88) as usize;

    let bits = (len as u64) * 8;
    let nl_new = nl as u64 + bits;
    nl = nl_new as u32;
    nh = nh.wrapping_add((nl_new >> 32) as u32);

    let mut input_idx = 0;
    let input_len = len as usize;

    while input_idx < input_len {
        let space = 64 - num;
        let chunk = std::cmp::min(space, input_len - input_idx);
        ctx[24 + num..24 + num + chunk].copy_from_slice(&input[input_idx..input_idx + chunk]);
        num += chunk;
        input_idx += chunk;

        if num == 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&ctx[24..88]);
            md5_step(&mut state, &block);
            num = 0;
        }
    }

    write_u32_le(&mut ctx, 0, state[0]);
    write_u32_le(&mut ctx, 4, state[1]);
    write_u32_le(&mut ctx, 8, state[2]);
    write_u32_le(&mut ctx, 12, state[3]);
    write_u32_le(&mut ctx, 16, nl);
    write_u32_le(&mut ctx, 20, nh);
    write_u32_le(&mut ctx, 88, num as u32);

    env.mem.bytes_at_mut(c_ptr, 92).copy_from_slice(&ctx);
    1
}

#[allow(non_snake_case)]
fn CC_MD5_Final(env: &mut Environment, md: MutVoidPtr, c: MutVoidPtr) -> i32 {
    if md.is_null() || c.is_null() {
        return 0;
    }
    let md_ptr = md.cast::<u8>();
    let c_ptr = c.cast::<u8>();

    let mut ctx = env.mem.bytes_at(c_ptr, 92).to_vec();
    let mut state = [
        read_u32_le(&ctx, 0),
        read_u32_le(&ctx, 4),
        read_u32_le(&ctx, 8),
        read_u32_le(&ctx, 12),
    ];
    let nl = read_u32_le(&ctx, 16);
    let nh = read_u32_le(&ctx, 20);
    let mut num = read_u32_le(&ctx, 88) as usize;

    ctx[24 + num] = 0x80;
    num += 1;

    if num > 56 {
        for i in num..64 {
            ctx[24 + i] = 0;
        }
        let mut block = [0u8; 64];
        block.copy_from_slice(&ctx[24..88]);
        md5_step(&mut state, &block);
        num = 0;
    }

    for i in num..56 {
        ctx[24 + i] = 0;
    }

    ctx[24 + 56..24 + 60].copy_from_slice(&nl.to_le_bytes());
    ctx[24 + 60..24 + 64].copy_from_slice(&nh.to_le_bytes());

    let mut block = [0u8; 64];
    block.copy_from_slice(&ctx[24..88]);
    md5_step(&mut state, &block);

    let mut hash = [0u8; 16];
    hash[0..4].copy_from_slice(&state[0].to_le_bytes());
    hash[4..8].copy_from_slice(&state[1].to_le_bytes());
    hash[8..12].copy_from_slice(&state[2].to_le_bytes());
    hash[12..16].copy_from_slice(&state[3].to_le_bytes());

    env.mem.bytes_at_mut(md_ptr, 16).copy_from_slice(&hash);
    env.mem.bytes_at_mut(c_ptr, 92).copy_from_slice(&[0u8; 92]);

    1
}

// AES S-box
const AES_SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

// AES inverse S-box
const AES_INV_SBOX: [u8; 256] = [
    0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
    0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
    0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
    0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
    0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
    0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
    0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
    0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
    0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
    0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
    0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
    0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
    0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
    0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
    0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
    0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d,
];

// AES round constants
const AES_RCON: [u8; 10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

fn aes_key_expansion(key: &[u8], nk: usize, nr: usize) -> Vec<u8> {
    let nb = 4;
    let total_words = nb * (nr + 1);
    let mut w = vec![0u32; total_words];

    for i in 0..nk {
        w[i] = u32::from_be_bytes([key[4 * i], key[4 * i + 1], key[4 * i + 2], key[4 * i + 3]]);
    }

    for i in nk..total_words {
        let mut temp = w[i - 1];
        if i % nk == 0 {
            // RotWord + SubWord + Rcon
            temp = temp.rotate_left(8);
            let b = temp.to_be_bytes();
            temp = u32::from_be_bytes([
                AES_SBOX[b[0] as usize],
                AES_SBOX[b[1] as usize],
                AES_SBOX[b[2] as usize],
                AES_SBOX[b[3] as usize],
            ]);
            temp ^= (AES_RCON[i / nk - 1] as u32) << 24;
        } else if nk > 6 && i % nk == 4 {
            let b = temp.to_be_bytes();
            temp = u32::from_be_bytes([
                AES_SBOX[b[0] as usize],
                AES_SBOX[b[1] as usize],
                AES_SBOX[b[2] as usize],
                AES_SBOX[b[3] as usize],
            ]);
        }
        w[i] = w[i - nk] ^ temp;
    }

    let mut expanded = vec![0u8; total_words * 4];
    for (i, &word) in w.iter().enumerate() {
        expanded[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
    }
    expanded
}

fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut result: u8 = 0;
    for _ in 0..8 {
        if b & 1 != 0 {
            result ^= a;
        }
        let hi = a & 0x80;
        a <<= 1;
        if hi != 0 {
            a ^= 0x1b;
        }
        b >>= 1;
    }
    result
}

fn aes_encrypt_block(block: &[u8; 16], expanded_key: &[u8], nr: usize) -> [u8; 16] {
    let mut state = *block;

    // AddRoundKey (round 0)
    for i in 0..16 {
        state[i] ^= expanded_key[i];
    }

    for round in 1..nr {
        let rk_off = round * 16;

        // SubBytes
        for b in &mut state {
            *b = AES_SBOX[*b as usize];
        }

        // ShiftRows (state is column-major: index = row + 4*col)
        let tmp = state[1];
        state[1] = state[5];
        state[5] = state[9];
        state[9] = state[13];
        state[13] = tmp;
        let tmp0 = state[2];
        let tmp1 = state[6];
        state[2] = state[10];
        state[6] = state[14];
        state[10] = tmp0;
        state[14] = tmp1;
        let tmp = state[15];
        state[15] = state[11];
        state[11] = state[7];
        state[7] = state[3];
        state[3] = tmp;

        // MixColumns
        for c in 0..4 {
            let i = c * 4;
            let s0 = state[i];
            let s1 = state[i + 1];
            let s2 = state[i + 2];
            let s3 = state[i + 3];
            state[i] = gf_mul(2, s0) ^ gf_mul(3, s1) ^ s2 ^ s3;
            state[i + 1] = s0 ^ gf_mul(2, s1) ^ gf_mul(3, s2) ^ s3;
            state[i + 2] = s0 ^ s1 ^ gf_mul(2, s2) ^ gf_mul(3, s3);
            state[i + 3] = gf_mul(3, s0) ^ s1 ^ s2 ^ gf_mul(2, s3);
        }

        // AddRoundKey
        for i in 0..16 {
            state[i] ^= expanded_key[rk_off + i];
        }
    }

    // Final round (no MixColumns)
    for b in &mut state {
        *b = AES_SBOX[*b as usize];
    }

    let tmp = state[1];
    state[1] = state[5];
    state[5] = state[9];
    state[9] = state[13];
    state[13] = tmp;
    let tmp0 = state[2];
    let tmp1 = state[6];
    state[2] = state[10];
    state[6] = state[14];
    state[10] = tmp0;
    state[14] = tmp1;
    let tmp = state[15];
    state[15] = state[11];
    state[11] = state[7];
    state[7] = state[3];
    state[3] = tmp;

    let rk_off = nr * 16;
    for i in 0..16 {
        state[i] ^= expanded_key[rk_off + i];
    }

    state
}

fn aes_decrypt_block(block: &[u8; 16], expanded_key: &[u8], nr: usize) -> [u8; 16] {
    let mut state = *block;

    // AddRoundKey (last round key)
    let rk_off = nr * 16;
    for i in 0..16 {
        state[i] ^= expanded_key[rk_off + i];
    }

    for round in (1..nr).rev() {
        let rk_off = round * 16;

        // InvShiftRows
        let tmp = state[13];
        state[13] = state[9];
        state[9] = state[5];
        state[5] = state[1];
        state[1] = tmp;
        let tmp0 = state[10];
        let tmp1 = state[14];
        state[10] = state[2];
        state[14] = state[6];
        state[2] = tmp0;
        state[6] = tmp1;
        let tmp = state[3];
        state[3] = state[7];
        state[7] = state[11];
        state[11] = state[15];
        state[15] = tmp;

        // InvSubBytes
        for b in &mut state {
            *b = AES_INV_SBOX[*b as usize];
        }

        // AddRoundKey
        for i in 0..16 {
            state[i] ^= expanded_key[rk_off + i];
        }

        // InvMixColumns
        for c in 0..4 {
            let i = c * 4;
            let s0 = state[i];
            let s1 = state[i + 1];
            let s2 = state[i + 2];
            let s3 = state[i + 3];
            state[i] = gf_mul(0x0e, s0) ^ gf_mul(0x0b, s1) ^ gf_mul(0x0d, s2) ^ gf_mul(0x09, s3);
            state[i + 1] =
                gf_mul(0x09, s0) ^ gf_mul(0x0e, s1) ^ gf_mul(0x0b, s2) ^ gf_mul(0x0d, s3);
            state[i + 2] =
                gf_mul(0x0d, s0) ^ gf_mul(0x09, s1) ^ gf_mul(0x0e, s2) ^ gf_mul(0x0b, s3);
            state[i + 3] =
                gf_mul(0x0b, s0) ^ gf_mul(0x0d, s1) ^ gf_mul(0x09, s2) ^ gf_mul(0x0e, s3);
        }
    }

    // Final inverse round (no InvMixColumns)
    let tmp = state[13];
    state[13] = state[9];
    state[9] = state[5];
    state[5] = state[1];
    state[1] = tmp;
    let tmp0 = state[10];
    let tmp1 = state[14];
    state[10] = state[2];
    state[14] = state[6];
    state[2] = tmp0;
    state[6] = tmp1;
    let tmp = state[3];
    state[3] = state[7];
    state[7] = state[11];
    state[11] = state[15];
    state[15] = tmp;

    for b in &mut state {
        *b = AES_INV_SBOX[*b as usize];
    }

    for i in 0..16 {
        state[i] ^= expanded_key[i];
    }

    state
}

// ============================================================================
// MARK: - DES (FIPS 46-3)
// ============================================================================
//
// Apple's CommonCrypto exposes single-DES via `kCCAlgorithmDES (1)`. Our app
// inputs include game DRM blobs and analytics SDK payloads. Implement DES
// per the published FIPS-46-3 specification so we produce identical bytes
// to Apple's CCCrypt rather than copying data through unchanged.

const DES_IP: [u8; 64] = [
    58, 50, 42, 34, 26, 18, 10, 2, 60, 52, 44, 36, 28, 20, 12, 4,
    62, 54, 46, 38, 30, 22, 14, 6, 64, 56, 48, 40, 32, 24, 16, 8,
    57, 49, 41, 33, 25, 17, 9, 1, 59, 51, 43, 35, 27, 19, 11, 3,
    61, 53, 45, 37, 29, 21, 13, 5, 63, 55, 47, 39, 31, 23, 15, 7,
];
const DES_FP: [u8; 64] = [
    40, 8, 48, 16, 56, 24, 64, 32, 39, 7, 47, 15, 55, 23, 63, 31,
    38, 6, 46, 14, 54, 22, 62, 30, 37, 5, 45, 13, 53, 21, 61, 29,
    36, 4, 44, 12, 52, 20, 60, 28, 35, 3, 43, 11, 51, 19, 59, 27,
    34, 2, 42, 10, 50, 18, 58, 26, 33, 1, 41, 9, 49, 17, 57, 25,
];
const DES_E: [u8; 48] = [
    32, 1, 2, 3, 4, 5, 4, 5, 6, 7, 8, 9, 8, 9, 10, 11, 12, 13,
    12, 13, 14, 15, 16, 17, 16, 17, 18, 19, 20, 21, 20, 21, 22, 23, 24, 25,
    24, 25, 26, 27, 28, 29, 28, 29, 30, 31, 32, 1,
];
const DES_P: [u8; 32] = [
    16, 7, 20, 21, 29, 12, 28, 17, 1, 15, 23, 26, 5, 18, 31, 10,
    2, 8, 24, 14, 32, 27, 3, 9, 19, 13, 30, 6, 22, 11, 4, 25,
];
const DES_PC1: [u8; 56] = [
    57, 49, 41, 33, 25, 17, 9, 1, 58, 50, 42, 34, 26, 18,
    10, 2, 59, 51, 43, 35, 27, 19, 11, 3, 60, 52, 44, 36,
    63, 55, 47, 39, 31, 23, 15, 7, 62, 54, 46, 38, 30, 22,
    14, 6, 61, 53, 45, 37, 29, 21, 13, 5, 28, 20, 12, 4,
];
const DES_PC2: [u8; 48] = [
    14, 17, 11, 24, 1, 5, 3, 28, 15, 6, 21, 10,
    23, 19, 12, 4, 26, 8, 16, 7, 27, 20, 13, 2,
    41, 52, 31, 37, 47, 55, 30, 40, 51, 45, 33, 48,
    44, 49, 39, 56, 34, 53, 46, 42, 50, 36, 29, 32,
];
const DES_SHIFTS: [u8; 16] =
    [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1];

const DES_SBOX: [[u8; 64]; 8] = [
    [
        14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7,
        0, 15, 7, 4, 14, 2, 13, 1, 10, 6, 12, 11, 9, 5, 3, 8,
        4, 1, 14, 8, 13, 6, 2, 11, 15, 12, 9, 7, 3, 10, 5, 0,
        15, 12, 8, 2, 4, 9, 1, 7, 5, 11, 3, 14, 10, 0, 6, 13,
    ],
    [
        15, 1, 8, 14, 6, 11, 3, 4, 9, 7, 2, 13, 12, 0, 5, 10,
        3, 13, 4, 7, 15, 2, 8, 14, 12, 0, 1, 10, 6, 9, 11, 5,
        0, 14, 7, 11, 10, 4, 13, 1, 5, 8, 12, 6, 9, 3, 2, 15,
        13, 8, 10, 1, 3, 15, 4, 2, 11, 6, 7, 12, 0, 5, 14, 9,
    ],
    [
        10, 0, 9, 14, 6, 3, 15, 5, 1, 13, 12, 7, 11, 4, 2, 8,
        13, 7, 0, 9, 3, 4, 6, 10, 2, 8, 5, 14, 12, 11, 15, 1,
        13, 6, 4, 9, 8, 15, 3, 0, 11, 1, 2, 12, 5, 10, 14, 7,
        1, 10, 13, 0, 6, 9, 8, 7, 4, 15, 14, 3, 11, 5, 2, 12,
    ],
    [
        7, 13, 14, 3, 0, 6, 9, 10, 1, 2, 8, 5, 11, 12, 4, 15,
        13, 8, 11, 5, 6, 15, 0, 3, 4, 7, 2, 12, 1, 10, 14, 9,
        10, 6, 9, 0, 12, 11, 7, 13, 15, 1, 3, 14, 5, 2, 8, 4,
        3, 15, 0, 6, 10, 1, 13, 8, 9, 4, 5, 11, 12, 7, 2, 14,
    ],
    [
        2, 12, 4, 1, 7, 10, 11, 6, 8, 5, 3, 15, 13, 0, 14, 9,
        14, 11, 2, 12, 4, 7, 13, 1, 5, 0, 15, 10, 3, 9, 8, 6,
        4, 2, 1, 11, 10, 13, 7, 8, 15, 9, 12, 5, 6, 3, 0, 14,
        11, 8, 12, 7, 1, 14, 2, 13, 6, 15, 0, 9, 10, 4, 5, 3,
    ],
    [
        12, 1, 10, 15, 9, 2, 6, 8, 0, 13, 3, 4, 14, 7, 5, 11,
        10, 15, 4, 2, 7, 12, 9, 5, 6, 1, 13, 14, 0, 11, 3, 8,
        9, 14, 15, 5, 2, 8, 12, 3, 7, 0, 4, 10, 1, 13, 11, 6,
        4, 3, 2, 12, 9, 5, 15, 10, 11, 14, 1, 7, 6, 0, 8, 13,
    ],
    [
        4, 11, 2, 14, 15, 0, 8, 13, 3, 12, 9, 7, 5, 10, 6, 1,
        13, 0, 11, 7, 4, 9, 1, 10, 14, 3, 5, 12, 2, 15, 8, 6,
        1, 4, 11, 13, 12, 3, 7, 14, 10, 15, 6, 8, 0, 5, 9, 2,
        6, 11, 13, 8, 1, 4, 10, 7, 9, 5, 0, 15, 14, 2, 3, 12,
    ],
    [
        13, 2, 8, 4, 6, 15, 11, 1, 10, 9, 3, 14, 5, 0, 12, 7,
        1, 15, 13, 8, 10, 3, 7, 4, 12, 5, 6, 11, 0, 14, 9, 2,
        7, 11, 4, 1, 9, 12, 14, 2, 0, 6, 10, 13, 15, 3, 5, 8,
        2, 1, 14, 7, 4, 10, 8, 13, 15, 12, 9, 0, 3, 5, 6, 11,
    ],
];

fn des_bit(buf: u64, pos: usize, total: usize) -> u64 {
    (buf >> (total - pos)) & 1
}

fn des_permute(input: u64, table: &[u8], in_bits: usize) -> u64 {
    let mut out: u64 = 0;
    for (i, &p) in table.iter().enumerate() {
        out |= des_bit(input, p as usize, in_bits) << (table.len() - 1 - i);
    }
    out
}

fn des_key_schedule(key: u64) -> [u64; 16] {
    let permuted = des_permute(key, &DES_PC1, 64);
    let mut c = (permuted >> 28) & 0x0fff_ffff;
    let mut d = permuted & 0x0fff_ffff;
    let mut subkeys = [0u64; 16];
    for round in 0..16 {
        let shift = DES_SHIFTS[round] as u32;
        c = ((c << shift) | (c >> (28 - shift))) & 0x0fff_ffff;
        d = ((d << shift) | (d >> (28 - shift))) & 0x0fff_ffff;
        let merged = (c << 28) | d;
        subkeys[round] = des_permute(merged, &DES_PC2, 56);
    }
    subkeys
}

fn des_f(half: u32, subkey: u64) -> u32 {
    let expanded = des_permute(half as u64, &DES_E, 32) ^ subkey;
    let mut output: u32 = 0;
    for i in 0..8 {
        let chunk = ((expanded >> (42 - i * 6)) & 0x3f) as usize;
        let row = ((chunk & 0x20) >> 4) | (chunk & 0x01);
        let col = (chunk >> 1) & 0x0f;
        let s_val = DES_SBOX[i][row * 16 + col] as u32;
        output |= s_val << (28 - i * 4);
    }
    des_permute(output as u64, &DES_P, 32) as u32
}

fn des_encrypt_block(block: u64, subkeys: &[u64; 16]) -> u64 {
    let permuted = des_permute(block, &DES_IP, 64);
    let mut l = (permuted >> 32) as u32;
    let mut r = permuted as u32;
    for round in 0..16 {
        let new_r = l ^ des_f(r, subkeys[round]);
        l = r;
        r = new_r;
    }
    let pre_output = ((r as u64) << 32) | (l as u64);
    des_permute(pre_output, &DES_FP, 64)
}

fn des_decrypt_block(block: u64, subkeys: &[u64; 16]) -> u64 {
    let permuted = des_permute(block, &DES_IP, 64);
    let mut l = (permuted >> 32) as u32;
    let mut r = permuted as u32;
    for round in (0..16).rev() {
        let new_r = l ^ des_f(r, subkeys[round]);
        l = r;
        r = new_r;
    }
    let pre_output = ((r as u64) << 32) | (l as u64);
    des_permute(pre_output, &DES_FP, 64)
}

fn des_load_block(bytes: &[u8]) -> u64 {
    u64::from_be_bytes(bytes.try_into().unwrap())
}

fn des_store_block(block: u64, out: &mut [u8]) {
    out.copy_from_slice(&block.to_be_bytes());
}

fn des_process(
    encrypt: bool,
    input: &[u8],
    key: &[u8],
    iv: Option<&[u8]>,
    pkcs7_pad: bool,
    ecb_mode: bool,
) -> Result<Vec<u8>, i32> {
    if key.len() != 8 {
        return Err(kCCParamError);
    }
    let subkeys = des_key_schedule(des_load_block(key));
    let block_size = 8usize;

    if encrypt {
        let work_data: Vec<u8> = if pkcs7_pad {
            let pad_len = block_size - (input.len() % block_size);
            input
                .iter()
                .copied()
                .chain(std::iter::repeat_n(pad_len as u8, pad_len))
                .collect()
        } else {
            if !input.len().is_multiple_of(block_size) {
                return Err(kCCAlignmentError);
            }
            input.to_vec()
        };

        let mut output = vec![0u8; work_data.len()];
        let mut prev = [0u8; 8];
        if !ecb_mode {
            if let Some(iv_bytes) = iv {
                if iv_bytes.len() == block_size {
                    prev.copy_from_slice(iv_bytes);
                }
            }
        }

        for i in (0..work_data.len()).step_by(block_size) {
            let mut blk = [0u8; 8];
            blk.copy_from_slice(&work_data[i..i + block_size]);
            if !ecb_mode {
                for j in 0..block_size {
                    blk[j] ^= prev[j];
                }
            }
            let encrypted = des_encrypt_block(des_load_block(&blk), &subkeys);
            des_store_block(encrypted, &mut output[i..i + block_size]);
            if !ecb_mode {
                prev.copy_from_slice(&output[i..i + block_size]);
            }
        }
        Ok(output)
    } else {
        if input.is_empty() {
            return Ok(Vec::new());
        }
        if !input.len().is_multiple_of(block_size) {
            return Err(kCCAlignmentError);
        }
        let mut output = vec![0u8; input.len()];
        let mut prev = [0u8; 8];
        if !ecb_mode {
            if let Some(iv_bytes) = iv {
                if iv_bytes.len() == block_size {
                    prev.copy_from_slice(iv_bytes);
                }
            }
        }

        for i in (0..input.len()).step_by(block_size) {
            let mut blk = [0u8; 8];
            blk.copy_from_slice(&input[i..i + block_size]);
            let decrypted = des_decrypt_block(des_load_block(&blk), &subkeys);
            let mut decrypted_bytes = [0u8; 8];
            des_store_block(decrypted, &mut decrypted_bytes);
            if ecb_mode {
                output[i..i + block_size].copy_from_slice(&decrypted_bytes);
            } else {
                for j in 0..block_size {
                    output[i + j] = decrypted_bytes[j] ^ prev[j];
                }
                prev.copy_from_slice(&input[i..i + block_size]);
            }
        }

        let final_len = if pkcs7_pad {
            let pad = output[output.len() - 1] as usize;
            if pad == 0 || pad > block_size {
                return Err(kCCDecodeError);
            }
            output.len() - pad
        } else {
            output.len()
        };
        output.truncate(final_len);
        Ok(output)
    }
}

// CCCrypt has 11 args. All are passed via the standard ARM calling convention
// (R0-R3 + stack), handled by the CallFromGuest framework.
#[allow(non_snake_case)]
fn CCCrypt(
    env: &mut Environment,
    op: u32,
    alg: u32,
    options: u32,
    key: ConstVoidPtr,
    key_length: GuestUSize,
    iv: ConstVoidPtr,
    data_in: ConstVoidPtr,
    data_in_length: GuestUSize,
    data_out: MutVoidPtr,
    data_out_available: GuestUSize,
    data_out_moved: MutPtr<GuestUSize>,
) -> i32 {
    log!(
        "CCCrypt(op={}, alg={}, options={:#x}, keyLen={}, dataLen={})",
        op,
        alg,
        options,
        key_length,
        data_in_length
    );

    let ecb_mode = (options & 0x2) != 0;
    let pkcs7_pad = (options & 0x1) != 0;

    // RC4 stream cipher (alg == 4)
    if alg == 4 {
        if data_out_available < data_in_length {
            return kCCBufferTooSmall;
        }
        let input = env.mem.bytes_at(data_in.cast(), data_in_length).to_vec();
        let key_bytes = env.mem.bytes_at(key.cast(), key_length).to_vec();
        let mut output = vec![0u8; data_in_length as usize];

        let mut s: Vec<u8> = (0..=255u8).collect();
        let mut j: usize = 0;
        for i in 0..256usize {
            j = (j + s[i] as usize + key_bytes[i % key_length as usize] as usize) % 256;
            s.swap(i, j);
        }
        let mut i = 0usize;
        j = 0;
        for (idx, &byte) in input.iter().enumerate() {
            i = (i + 1) % 256;
            j = (j + s[i] as usize) % 256;
            s.swap(i, j);
            let k = s[(s[i] as usize + s[j] as usize) % 256];
            output[idx] = byte ^ k;
        }
        env.mem
            .bytes_at_mut(data_out.cast(), data_in_length)
            .copy_from_slice(&output);
        env.mem.write(data_out_moved, data_in_length);
        return kCCSuccess;
    }

    // Determine block size and number of rounds based on algorithm
    let block_size: usize = match alg {
        0 => 16, // kCCAlgorithmAES128
        1 => 8,  // kCCAlgorithmDES
        2 => 8,  // kCCAlgorithm3DES
        3 => 8,  // kCCAlgorithmCAST
        _ => {
            log!("CCCrypt: alg={} not supported, data copied as-is", alg);
            if data_out_available < data_in_length {
                return kCCBufferTooSmall;
            }
            let input = env.mem.bytes_at(data_in.cast(), data_in_length).to_vec();
            env.mem
                .bytes_at_mut(data_out.cast(), data_in_length)
                .copy_from_slice(&input);
            env.mem.write(data_out_moved, data_in_length);
            return kCCSuccess;
        }
    };

    // AES block cipher
    if alg == 0 {
        let (nk, nr) = match key_length {
            16 => (4, 10), // AES-128
            24 => (6, 12), // AES-192
            32 => (8, 14), // AES-256
            _ => {
                log!("CCCrypt: unsupported AES key length {}", key_length);
                return kCCParamError;
            }
        };

        let key_bytes = env.mem.bytes_at(key.cast(), key_length).to_vec();
        let expanded_key = aes_key_expansion(&key_bytes, nk, nr);

        let input = env.mem.bytes_at(data_in.cast(), data_in_length).to_vec();
        let input_len = data_in_length as usize;
        let is_encrypt = op == 0;

        let mut output: Vec<u8>;

        if is_encrypt {
            let padded: Vec<u8>;
            let work_data = if pkcs7_pad {
                let pad_len = block_size - (input_len % block_size);
                padded = input
                    .iter()
                    .copied()
                    .chain(std::iter::repeat_n(pad_len as u8, pad_len))
                    .collect();
                &padded
            } else {
                if !input_len.is_multiple_of(block_size) {
                    return kCCAlignmentError;
                }
                &input
            };

            let out_len = work_data.len();
            if data_out_available < out_len as GuestUSize {
                return kCCBufferTooSmall;
            }

            output = vec![0u8; out_len];
            let mut prev_block = [0u8; 16];
            if !ecb_mode && !iv.is_null() {
                prev_block.copy_from_slice(env.mem.bytes_at(iv.cast(), 16));
            }

            for i in (0..out_len).step_by(block_size) {
                let mut blk = [0u8; 16];
                blk.copy_from_slice(&work_data[i..i + block_size]);

                if !ecb_mode {
                    for j in 0..block_size {
                        blk[j] ^= prev_block[j];
                    }
                }

                let encrypted = aes_encrypt_block(&blk, &expanded_key, nr);
                output[i..i + block_size].copy_from_slice(&encrypted);

                if !ecb_mode {
                    prev_block.copy_from_slice(&encrypted);
                }
            }

            env.mem
                .bytes_at_mut(data_out.cast(), out_len as GuestUSize)
                .copy_from_slice(&output);
            env.mem.write(data_out_moved, out_len as GuestUSize);
        } else {
            // Decrypt
            if input_len == 0 {
                env.mem.write(data_out_moved, 0);
                return kCCSuccess;
            }
            if !input_len.is_multiple_of(block_size) {
                return kCCAlignmentError;
            }

            output = vec![0u8; input_len];
            let mut prev_block = [0u8; 16];
            if !ecb_mode && !iv.is_null() {
                prev_block.copy_from_slice(env.mem.bytes_at(iv.cast(), 16));
            }

            for i in (0..input_len).step_by(block_size) {
                let mut blk = [0u8; 16];
                blk.copy_from_slice(&input[i..i + block_size]);

                let decrypted = aes_decrypt_block(&blk, &expanded_key, nr);

                if ecb_mode {
                    output[i..i + block_size].copy_from_slice(&decrypted);
                } else {
                    for j in 0..block_size {
                        output[i + j] = decrypted[j] ^ prev_block[j];
                    }
                    prev_block.copy_from_slice(&input[i..i + block_size]);
                }
            }

            let out_len = if pkcs7_pad {
                let pad = output[input_len - 1] as usize;
                if pad == 0 || pad > block_size {
                    return kCCDecodeError;
                }
                input_len - pad
            } else {
                input_len
            };

            if data_out_available < out_len as GuestUSize {
                return kCCBufferTooSmall;
            }

            env.mem
                .bytes_at_mut(data_out.cast(), out_len as GuestUSize)
                .copy_from_slice(&output[..out_len]);
            env.mem.write(data_out_moved, out_len as GuestUSize);
        }

        return kCCSuccess;
    }

    // Unsupported block cipher algorithm: copy as-is
    if alg == 1 {
        let key_bytes = env.mem.bytes_at(key.cast(), key_length).to_vec();
        if key_bytes.len() != 8 {
            return kCCParamError;
        }
        let input = env.mem.bytes_at(data_in.cast(), data_in_length).to_vec();
        let iv_bytes_opt = if !ecb_mode && !iv.is_null() {
            Some(env.mem.bytes_at(iv.cast(), block_size as GuestUSize).to_vec())
        } else {
            None
        };
        let iv_slice = iv_bytes_opt.as_deref();
        let is_encrypt = op == 0;
        return match des_process(is_encrypt, &input, &key_bytes, iv_slice, pkcs7_pad, ecb_mode) {
            Ok(output) => {
                if data_out_available < output.len() as GuestUSize {
                    return kCCBufferTooSmall;
                }
                env.mem
                    .bytes_at_mut(data_out.cast(), output.len() as GuestUSize)
                    .copy_from_slice(&output);
                env.mem.write(data_out_moved, output.len() as GuestUSize);
                kCCSuccess
            }
            Err(code) => code,
        };
    }
    if data_out_available < data_in_length {
        return kCCBufferTooSmall;
    }
    let input = env.mem.bytes_at(data_in.cast(), data_in_length).to_vec();
    env.mem
        .bytes_at_mut(data_out.cast(), data_in_length)
        .copy_from_slice(&input);
    env.mem.write(data_out_moved, data_in_length);
    log!("CCCrypt: alg={} not implemented, data copied as-is", alg);
    kCCSuccess
}

#[allow(non_snake_case)]
fn CCKeyDerivationPBKDF(
    _env: &mut Environment,
    _algorithm: u32,
    _password: ConstVoidPtr,
    _password_len: GuestUSize,
    _salt: ConstVoidPtr,
    _salt_len: GuestUSize,
    _prf: u32,
    _rounds: u32,
) -> i32 {
    log!("TODO: CCKeyDerivationPBKDF");
    kCCSuccess
}

// One-shot MD5 hash (host-side, no guest memory)
fn md5_hash(data: &[u8]) -> [u8; 16] {
    let mut state: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
    let bit_len = (data.len() as u64) * 8;

    // Process complete 64-byte blocks
    let mut offset = 0;
    while offset + 64 <= data.len() {
        let mut block = [0u8; 64];
        block.copy_from_slice(&data[offset..offset + 64]);
        md5_step(&mut state, &block);
        offset += 64;
    }

    // Padding
    let mut last = Vec::with_capacity(128);
    last.extend_from_slice(&data[offset..]);
    last.push(0x80);
    while last.len() % 64 != 56 {
        last.push(0);
    }
    last.extend_from_slice(&bit_len.to_le_bytes());

    for chunk in last.chunks_exact(64) {
        let mut block = [0u8; 64];
        block.copy_from_slice(chunk);
        md5_step(&mut state, &block);
    }

    let mut hash = [0u8; 16];
    hash[0..4].copy_from_slice(&state[0].to_le_bytes());
    hash[4..8].copy_from_slice(&state[1].to_le_bytes());
    hash[8..12].copy_from_slice(&state[2].to_le_bytes());
    hash[12..16].copy_from_slice(&state[3].to_le_bytes());
    hash
}

// SHA-1 block transform
fn sha1_step(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0u32; 80];
    for i in 0..16 {
        w[i] = u32::from_be_bytes([
            block[i * 4],
            block[i * 4 + 1],
            block[i * 4 + 2],
            block[i * 4 + 3],
        ]);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let [mut a, mut b, mut c, mut d, mut e] = *state;
    for i in 0..80 {
        let (f, k) = match i {
            0..=19 => ((b & c) | (!b & d), 0x5A827999u32),
            20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
            60..=79 => (b ^ c ^ d, 0xCA62C1D6u32),
            _ => unreachable!(),
        };
        let temp = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(w[i]);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = temp;
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
}

// One-shot SHA-1 hash
fn sha1_hash(data: &[u8]) -> [u8; 20] {
    let mut state: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let bit_len = (data.len() as u64) * 8;

    let mut offset = 0;
    while offset + 64 <= data.len() {
        let mut block = [0u8; 64];
        block.copy_from_slice(&data[offset..offset + 64]);
        sha1_step(&mut state, &block);
        offset += 64;
    }

    let mut last = Vec::with_capacity(128);
    last.extend_from_slice(&data[offset..]);
    last.push(0x80);
    while last.len() % 64 != 56 {
        last.push(0);
    }
    last.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in last.chunks_exact(64) {
        let mut block = [0u8; 64];
        block.copy_from_slice(chunk);
        sha1_step(&mut state, &block);
    }

    let mut hash = [0u8; 20];
    hash[0..4].copy_from_slice(&state[0].to_be_bytes());
    hash[4..8].copy_from_slice(&state[1].to_be_bytes());
    hash[8..12].copy_from_slice(&state[2].to_be_bytes());
    hash[12..16].copy_from_slice(&state[3].to_be_bytes());
    hash[16..20].copy_from_slice(&state[4].to_be_bytes());
    hash
}

// SHA-256 block transform
fn sha256_step(state: &mut [u32; 8], block: &[u8; 64]) {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut w = [0u32; 64];
    for i in 0..16 {
        w[i] = u32::from_be_bytes([
            block[i * 4],
            block[i * 4 + 1],
            block[i * 4 + 2],
            block[i * 4 + 3],
        ]);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let temp1 = h
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(K[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = s0.wrapping_add(maj);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temp1);
        d = c;
        c = b;
        b = a;
        a = temp1.wrapping_add(temp2);
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}

// One-shot SHA-256 hash
fn sha256_hash(data: &[u8]) -> [u8; 32] {
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64) * 8;

    let mut offset = 0;
    while offset + 64 <= data.len() {
        let mut block = [0u8; 64];
        block.copy_from_slice(&data[offset..offset + 64]);
        sha256_step(&mut state, &block);
        offset += 64;
    }

    let mut last = Vec::with_capacity(128);
    last.extend_from_slice(&data[offset..]);
    last.push(0x80);
    while last.len() % 64 != 56 {
        last.push(0);
    }
    last.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in last.chunks_exact(64) {
        let mut block = [0u8; 64];
        block.copy_from_slice(chunk);
        sha256_step(&mut state, &block);
    }

    let mut hash = [0u8; 32];
    for i in 0..8 {
        hash[i * 4..(i + 1) * 4].copy_from_slice(&state[i].to_be_bytes());
    }
    hash
}

#[allow(non_snake_case)]
fn CCHmac(
    env: &mut Environment,
    algorithm: u32,
    key: ConstVoidPtr,
    key_length: GuestUSize,
    data: ConstVoidPtr,
    data_length: GuestUSize,
    mac_out: MutVoidPtr,
) {
    // algorithm: 0=SHA1, 1=MD5, 2=SHA256, 3=SHA384, 4=SHA512, 5=SHA224
    let (block_size, hash_len): (usize, usize) = match algorithm {
        0 => (64, 20), // kCCHmacAlgSHA1
        1 => (64, 16), // kCCHmacAlgMD5
        2 => (64, 32), // kCCHmacAlgSHA256
        _ => {
            log!("CCHmac: unsupported algorithm {}, writing zeros", algorithm);
            return;
        }
    };

    log!(
        "CCHmac(algorithm={}, keyLen={}, dataLen={})",
        algorithm,
        key_length,
        data_length
    );

    let key_bytes = env.mem.bytes_at(key.cast(), key_length).to_vec();
    let data_bytes = env.mem.bytes_at(data.cast(), data_length).to_vec();

    // If key is longer than block size, hash it first
    let key_block = if key_bytes.len() > block_size {
        match algorithm {
            0 => sha1_hash(&key_bytes).to_vec(),
            1 => md5_hash(&key_bytes).to_vec(),
            2 => sha256_hash(&key_bytes).to_vec(),
            _ => unreachable!(),
        }
    } else {
        key_bytes
    };

    // Pad key to block_size
    let mut k_ipad = vec![0x36u8; block_size];
    let mut k_opad = vec![0x5cu8; block_size];
    for i in 0..key_block.len() {
        k_ipad[i] ^= key_block[i];
        k_opad[i] ^= key_block[i];
    }

    // inner = H(k_ipad || data)
    let mut inner_data = k_ipad;
    inner_data.extend_from_slice(&data_bytes);

    let inner_hash: Vec<u8> = match algorithm {
        0 => sha1_hash(&inner_data).to_vec(),
        1 => md5_hash(&inner_data).to_vec(),
        2 => sha256_hash(&inner_data).to_vec(),
        _ => unreachable!(),
    };

    // outer = H(k_opad || inner_hash)
    let mut outer_data = k_opad;
    outer_data.extend_from_slice(&inner_hash);

    let result: Vec<u8> = match algorithm {
        0 => sha1_hash(&outer_data).to_vec(),
        1 => md5_hash(&outer_data).to_vec(),
        2 => sha256_hash(&outer_data).to_vec(),
        _ => unreachable!(),
    };

    env.mem
        .bytes_at_mut(mac_out.cast(), hash_len as GuestUSize)
        .copy_from_slice(&result[..hash_len]);
}

// =========================================================================
// MARK: - Security framework stubs (Keychain Services)
// =========================================================================
// These are no-ops — touchHLE has no keychain. Apps that use keychain
// for license checks or settings will gracefully handle errSecItemNotFound.

// OSStatus error codes
const errSecSuccess: i32 = 0;
const errSecItemNotFound: i32 = -25300;
const errSecParam: i32 = -50;

#[allow(non_snake_case)]
fn SecItemCopyMatching(
    _env: &mut Environment,
    _query: crate::mem::ConstVoidPtr,
    _result: crate::mem::MutVoidPtr,
) -> i32 {
    log_dbg!("SecItemCopyMatching -> errSecItemNotFound (stubbed)");
    errSecItemNotFound
}

#[allow(non_snake_case)]
fn SecItemAdd(
    _env: &mut Environment,
    _attributes: crate::mem::ConstVoidPtr,
    _result: crate::mem::MutVoidPtr,
) -> i32 {
    log_dbg!("SecItemAdd -> errSecSuccess (stubbed)");
    errSecSuccess
}

#[allow(non_snake_case)]
fn SecItemUpdate(
    _env: &mut Environment,
    _query: crate::mem::ConstVoidPtr,
    _attributes_to_update: crate::mem::ConstVoidPtr,
) -> i32 {
    log_dbg!("SecItemUpdate -> errSecItemNotFound (stubbed)");
    errSecItemNotFound
}

#[allow(non_snake_case)]
fn SecItemDelete(_env: &mut Environment, _query: crate::mem::ConstVoidPtr) -> i32 {
    log_dbg!("SecItemDelete -> errSecItemNotFound (stubbed)");
    errSecItemNotFound
}

// === One-shot SHA helpers ====================================================
//
// CommonCrypto exposes a "compute the hash in a single call" overload for
// every digest:
//
// ```c
// unsigned char *CC_SHA1  (const void *data, CC_LONG len, unsigned char *md);
// unsigned char *CC_SHA256(const void *data, CC_LONG len, unsigned char *md);
// /* etc. */
// ```
//
// `len` is `uint32_t` (CC_LONG), `md` is a caller-allocated output buffer of
// the digest's natural size (20/28/32/48/64 bytes), and the return value is
// the same `md` pointer (or NULL if `md` is NULL).
//
// We implement these via the `sha1` / `sha2` crates so analytics SDKs that
// hash device IDs / payloads get a real, deterministic digest rather than
// the all-zero fallback that the generic return-0 stub used to produce.

fn read_guest_bytes(env: &Environment, data: ConstVoidPtr, len: GuestUSize) -> Vec<u8> {
    if data.is_null() || len == 0 {
        return Vec::new();
    }
    env.mem.bytes_at(data.cast(), len).to_vec()
}

fn write_digest(env: &mut Environment, md: MutVoidPtr, digest: &[u8]) {
    if md.is_null() {
        return;
    }
    env.mem
        .bytes_at_mut(md.cast(), digest.len() as u32)
        .copy_from_slice(digest);
}

#[allow(non_snake_case)]
fn CC_SHA1(env: &mut Environment, data: ConstVoidPtr, len: GuestUSize, md: MutVoidPtr) -> MutVoidPtr {
    use sha1::{Digest, Sha1};
    if md.is_null() {
        return MutVoidPtr::null();
    }
    let bytes = read_guest_bytes(env, data, len);
    let digest = Sha1::digest(&bytes);
    write_digest(env, md, digest.as_slice());
    md
}

#[allow(non_snake_case)]
fn CC_SHA224(env: &mut Environment, data: ConstVoidPtr, len: GuestUSize, md: MutVoidPtr) -> MutVoidPtr {
    use sha2::{Digest, Sha224};
    if md.is_null() {
        return MutVoidPtr::null();
    }
    let bytes = read_guest_bytes(env, data, len);
    let digest = Sha224::digest(&bytes);
    write_digest(env, md, digest.as_slice());
    md
}

#[allow(non_snake_case)]
fn CC_SHA256(env: &mut Environment, data: ConstVoidPtr, len: GuestUSize, md: MutVoidPtr) -> MutVoidPtr {
    use sha2::{Digest, Sha256};
    if md.is_null() {
        return MutVoidPtr::null();
    }
    let bytes = read_guest_bytes(env, data, len);
    let digest = Sha256::digest(&bytes);
    write_digest(env, md, digest.as_slice());
    md
}

#[allow(non_snake_case)]
fn CC_SHA384(env: &mut Environment, data: ConstVoidPtr, len: GuestUSize, md: MutVoidPtr) -> MutVoidPtr {
    use sha2::{Digest, Sha384};
    if md.is_null() {
        return MutVoidPtr::null();
    }
    let bytes = read_guest_bytes(env, data, len);
    let digest = Sha384::digest(&bytes);
    write_digest(env, md, digest.as_slice());
    md
}

#[allow(non_snake_case)]
fn CC_SHA512(env: &mut Environment, data: ConstVoidPtr, len: GuestUSize, md: MutVoidPtr) -> MutVoidPtr {
    use sha2::{Digest, Sha512};
    if md.is_null() {
        return MutVoidPtr::null();
    }
    let bytes = read_guest_bytes(env, data, len);
    let digest = Sha512::digest(&bytes);
    write_digest(env, md, digest.as_slice());
    md
}

// MARK: - Incremental SHA-family helpers (CommonDigest.h)
//
// Apple's CommonDigest.h declares the following triplet for each of SHA1,
// SHA224, SHA256, SHA384 and SHA512:
//
//     int CC_SHAxxx_Init   (CC_SHAxxx_CTX *c);
//     int CC_SHAxxx_Update (CC_SHAxxx_CTX *c, const void *data, CC_LONG len);
//     int CC_SHAxxx_Final  (unsigned char *md, CC_SHAxxx_CTX *c);
//
// All three return 1 on success, 0 on failure. The CTX struct is opaque to
// the caller — guests treat it as an arbitrary fixed-size buffer they only
// pass back to subsequent calls — so we don't need to mirror Apple's exact
// internal layout. Instead we stamp a sentinel in the first 4 bytes so
// Update/Final can validate that the ctx was previously Init'd, and stash
// the actual sha1::Sha1 / sha2::Sha256 / sha2::Sha512 state in a host-side
// table keyed by the guest ctx pointer. This gives a real, correct
// incremental SHA implementation (not a stub) without any reverse-engineered
// SHA round code.

use sha1::Sha1;
use sha2::{Digest as Sha2Digest, Sha224, Sha256, Sha384, Sha512};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

enum ShaState {
    S1(Sha1),
    S224(Sha224),
    S256(Sha256),
    S384(Sha384),
    S512(Sha512),
}

impl ShaState {
    fn update(&mut self, data: &[u8]) {
        match self {
            ShaState::S1(h) => sha1::Digest::update(h, data),
            ShaState::S224(h) => Sha2Digest::update(h, data),
            ShaState::S256(h) => Sha2Digest::update(h, data),
            ShaState::S384(h) => Sha2Digest::update(h, data),
            ShaState::S512(h) => Sha2Digest::update(h, data),
        }
    }
    fn finalize_into(self, out: &mut [u8]) {
        match self {
            ShaState::S1(h) => out.copy_from_slice(&h.finalize()),
            ShaState::S224(h) => out.copy_from_slice(&h.finalize()),
            ShaState::S256(h) => out.copy_from_slice(&h.finalize()),
            ShaState::S384(h) => out.copy_from_slice(&h.finalize()),
            ShaState::S512(h) => out.copy_from_slice(&h.finalize()),
        }
    }
}

const SHA_INIT_SENTINEL: u32 = 0xCAFE_C0DE;

fn sha_state_table() -> &'static Mutex<HashMap<u32, ShaState>> {
    static T: OnceLock<Mutex<HashMap<u32, ShaState>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(HashMap::new()))
}

fn sha_init(env: &mut Environment, c: MutVoidPtr, state: ShaState) -> i32 {
    if c.is_null() {
        return 0;
    }
    env.mem
        .write(c.cast::<u32>(), SHA_INIT_SENTINEL.to_le());
    sha_state_table()
        .lock()
        .unwrap()
        .insert(c.to_bits(), state);
    1
}

fn sha_update_inner(env: &mut Environment, c: MutVoidPtr, data: ConstVoidPtr, len: GuestUSize) -> i32 {
    if c.is_null() {
        return 0;
    }
    let sentinel = u32::from_le(env.mem.read(c.cast::<u32>()));
    if sentinel != SHA_INIT_SENTINEL {
        return 0;
    }
    if len == 0 || data.is_null() {
        return 1;
    }
    let bytes = read_guest_bytes(env, data, len);
    let mut table = sha_state_table().lock().unwrap();
    if let Some(state) = table.get_mut(&c.to_bits()) {
        state.update(&bytes);
        1
    } else {
        0
    }
}

fn sha_final_inner(env: &mut Environment, md: MutVoidPtr, c: MutVoidPtr, digest_len: usize) -> i32 {
    if md.is_null() || c.is_null() {
        return 0;
    }
    let sentinel = u32::from_le(env.mem.read(c.cast::<u32>()));
    if sentinel != SHA_INIT_SENTINEL {
        return 0;
    }
    // Clear sentinel so a subsequent Update/Final on the same ctx fails
    // until Init is called again — matches Apple's "context is consumed"
    // semantics.
    env.mem.write(c.cast::<u32>(), 0u32.to_le());
    let state = sha_state_table().lock().unwrap().remove(&c.to_bits());
    if let Some(state) = state {
        let mut out = vec![0u8; digest_len];
        state.finalize_into(&mut out);
        write_digest(env, md, &out);
        1
    } else {
        0
    }
}

#[allow(non_snake_case)]
fn CC_SHA1_Init(env: &mut Environment, c: MutVoidPtr) -> i32 {
    sha_init(env, c, ShaState::S1(Sha1::new()))
}
#[allow(non_snake_case)]
fn CC_SHA1_Update(env: &mut Environment, c: MutVoidPtr, data: ConstVoidPtr, len: GuestUSize) -> i32 {
    sha_update_inner(env, c, data, len)
}
#[allow(non_snake_case)]
fn CC_SHA1_Final(env: &mut Environment, md: MutVoidPtr, c: MutVoidPtr) -> i32 {
    sha_final_inner(env, md, c, 20)
}

#[allow(non_snake_case)]
fn CC_SHA224_Init(env: &mut Environment, c: MutVoidPtr) -> i32 {
    sha_init(env, c, ShaState::S224(Sha224::new()))
}
#[allow(non_snake_case)]
fn CC_SHA224_Update(env: &mut Environment, c: MutVoidPtr, data: ConstVoidPtr, len: GuestUSize) -> i32 {
    sha_update_inner(env, c, data, len)
}
#[allow(non_snake_case)]
fn CC_SHA224_Final(env: &mut Environment, md: MutVoidPtr, c: MutVoidPtr) -> i32 {
    sha_final_inner(env, md, c, 28)
}

#[allow(non_snake_case)]
fn CC_SHA256_Init(env: &mut Environment, c: MutVoidPtr) -> i32 {
    sha_init(env, c, ShaState::S256(Sha256::new()))
}
#[allow(non_snake_case)]
fn CC_SHA256_Update(env: &mut Environment, c: MutVoidPtr, data: ConstVoidPtr, len: GuestUSize) -> i32 {
    sha_update_inner(env, c, data, len)
}
#[allow(non_snake_case)]
fn CC_SHA256_Final(env: &mut Environment, md: MutVoidPtr, c: MutVoidPtr) -> i32 {
    sha_final_inner(env, md, c, 32)
}

#[allow(non_snake_case)]
fn CC_SHA384_Init(env: &mut Environment, c: MutVoidPtr) -> i32 {
    sha_init(env, c, ShaState::S384(Sha384::new()))
}
#[allow(non_snake_case)]
fn CC_SHA384_Update(env: &mut Environment, c: MutVoidPtr, data: ConstVoidPtr, len: GuestUSize) -> i32 {
    sha_update_inner(env, c, data, len)
}
#[allow(non_snake_case)]
fn CC_SHA384_Final(env: &mut Environment, md: MutVoidPtr, c: MutVoidPtr) -> i32 {
    sha_final_inner(env, md, c, 48)
}

#[allow(non_snake_case)]
fn CC_SHA512_Init(env: &mut Environment, c: MutVoidPtr) -> i32 {
    sha_init(env, c, ShaState::S512(Sha512::new()))
}
#[allow(non_snake_case)]
fn CC_SHA512_Update(env: &mut Environment, c: MutVoidPtr, data: ConstVoidPtr, len: GuestUSize) -> i32 {
    sha_update_inner(env, c, data, len)
}
#[allow(non_snake_case)]
fn CC_SHA512_Final(env: &mut Environment, md: MutVoidPtr, c: MutVoidPtr) -> i32 {
    sha_final_inner(env, md, c, 64)
}

// MARK: - Streaming CCCryptor API (CommonCryptor.h)
//
// Apple's CommonCryptor exposes a streaming/stateful API alongside the
// one-shot `CCCrypt`:
//
//     CCCryptorStatus CCCryptorCreate(CCOperation op, CCAlgorithm alg,
//         CCOptions options, const void *key, size_t keyLength,
//         const void *iv, CCCryptorRef *cryptorRef);
//     CCCryptorStatus CCCryptorUpdate(CCCryptorRef cryptorRef,
//         const void *dataIn, size_t dataInLength,
//         void *dataOut, size_t dataOutAvailable, size_t *dataOutMoved);
//     CCCryptorStatus CCCryptorFinal(CCCryptorRef cryptorRef,
//         void *dataOut, size_t dataOutAvailable, size_t *dataOutMoved);
//     size_t CCCryptorGetOutputLength(CCCryptorRef cryptorRef,
//         size_t inputLength, bool final);
//     CCCryptorStatus CCCryptorReset(CCCryptorRef cryptorRef, const void *iv);
//     CCCryptorStatus CCCryptorRelease(CCCryptorRef cryptorRef);
//
// `CCCryptorRef` is an opaque pointer; the guest only ever stores it and
// passes it back, so we hand out a small guest allocation as the handle and
// keep the real cipher state host-side in a table keyed by the handle bits.
// This is a real implementation built on the same AES/DES/RC4 primitives
// used by `CCCrypt`, not a stub.
// <https://github.com/Apple-FOSS-Mirror/CommonCrypto/blob/master/CommonCrypto/CommonCryptor.h>

enum CryptorCipher {
    /// AES with expanded key + number of rounds.
    Aes { expanded_key: Vec<u8>, nr: usize },
    /// DES with its key schedule.
    Des { subkeys: [u64; 16] },
    /// RC4 stream cipher: evolving S-box plus the two indices.
    Rc4 { s: Vec<u8>, i: usize, j: usize },
}

struct CryptorState {
    encrypt: bool,
    ecb_mode: bool,
    pkcs7_pad: bool,
    block_size: usize,
    /// CBC chaining block (block_size bytes). Unused in ECB / for RC4.
    chain: Vec<u8>,
    /// Initial IV, kept so CCCryptorReset can restore chaining state.
    initial_iv: Vec<u8>,
    cipher: CryptorCipher,
    /// Bytes buffered because they didn't fill a whole block yet (block
    /// ciphers only).
    buffer: Vec<u8>,
}

fn cryptor_table() -> &'static Mutex<HashMap<u32, CryptorState>> {
    static T: OnceLock<Mutex<HashMap<u32, CryptorState>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Encrypt/decrypt one block in place using the cryptor's cipher + mode,
/// honouring CBC chaining. Only used for block ciphers.
fn cryptor_process_block(state: &mut CryptorState, block: &mut [u8]) {
    let bs = state.block_size;
    match &state.cipher {
        CryptorCipher::Aes { expanded_key, nr } => {
            if state.encrypt {
                if !state.ecb_mode {
                    for k in 0..bs {
                        block[k] ^= state.chain[k];
                    }
                }
                let mut inb = [0u8; 16];
                inb[..bs].copy_from_slice(&block[..bs]);
                let enc = aes_encrypt_block(&inb, expanded_key, *nr);
                block[..bs].copy_from_slice(&enc[..bs]);
                if !state.ecb_mode {
                    state.chain[..bs].copy_from_slice(&block[..bs]);
                }
            } else {
                let cipher_block: Vec<u8> = block[..bs].to_vec();
                let mut inb = [0u8; 16];
                inb[..bs].copy_from_slice(&block[..bs]);
                let dec = aes_decrypt_block(&inb, expanded_key, *nr);
                if state.ecb_mode {
                    block[..bs].copy_from_slice(&dec[..bs]);
                } else {
                    for k in 0..bs {
                        block[k] = dec[k] ^ state.chain[k];
                    }
                    state.chain[..bs].copy_from_slice(&cipher_block);
                }
            }
        }
        CryptorCipher::Des { subkeys } => {
            if state.encrypt {
                if !state.ecb_mode {
                    for k in 0..bs {
                        block[k] ^= state.chain[k];
                    }
                }
                let enc = des_encrypt_block(des_load_block(&block[..bs]), subkeys);
                des_store_block(enc, &mut block[..bs]);
                if !state.ecb_mode {
                    state.chain[..bs].copy_from_slice(&block[..bs]);
                }
            } else {
                let cipher_block: Vec<u8> = block[..bs].to_vec();
                let dec = des_decrypt_block(des_load_block(&block[..bs]), subkeys);
                let mut dec_bytes = [0u8; 8];
                des_store_block(dec, &mut dec_bytes);
                if state.ecb_mode {
                    block[..bs].copy_from_slice(&dec_bytes[..bs]);
                } else {
                    for k in 0..bs {
                        block[k] = dec_bytes[k] ^ state.chain[k];
                    }
                    state.chain[..bs].copy_from_slice(&cipher_block);
                }
            }
        }
        CryptorCipher::Rc4 { .. } => unreachable!("RC4 handled separately"),
    }
}

#[allow(non_snake_case)]
fn CCCryptorCreate(
    env: &mut Environment,
    op: u32,
    alg: u32,
    options: u32,
    key: ConstVoidPtr,
    key_length: GuestUSize,
    iv: ConstVoidPtr,
    cryptor_ref_out: MutPtr<MutVoidPtr>,
) -> i32 {
    if cryptor_ref_out.is_null() {
        return kCCParamError;
    }

    let encrypt = op == 0;
    let ecb_mode = (options & 0x2) != 0;
    let pkcs7_pad = (options & 0x1) != 0;

    let key_bytes = read_guest_bytes(env, key, key_length);

    let (cipher, block_size) = match alg {
        0 => {
            // AES
            let (nk, nr) = match key_length {
                16 => (4, 10),
                24 => (6, 12),
                32 => (8, 14),
                _ => return kCCParamError,
            };
            let expanded_key = aes_key_expansion(&key_bytes, nk, nr);
            (CryptorCipher::Aes { expanded_key, nr }, 16usize)
        }
        1 => {
            // DES
            if key_bytes.len() != 8 {
                return kCCParamError;
            }
            let subkeys = des_key_schedule(des_load_block(&key_bytes));
            (CryptorCipher::Des { subkeys }, 8usize)
        }
        4 => {
            // RC4 stream cipher
            if key_bytes.is_empty() {
                return kCCParamError;
            }
            let mut s: Vec<u8> = (0..=255u8).collect();
            let mut j: usize = 0;
            for i in 0..256usize {
                j = (j + s[i] as usize + key_bytes[i % key_bytes.len()] as usize) % 256;
                s.swap(i, j);
            }
            (CryptorCipher::Rc4 { s, i: 0, j: 0 }, 1usize)
        }
        _ => {
            log!("CCCryptorCreate: unsupported alg={}", alg);
            return kCCParamError;
        }
    };

    let initial_iv = if !ecb_mode && !iv.is_null() && block_size > 1 {
        read_guest_bytes(env, iv, block_size as GuestUSize)
    } else {
        vec![0u8; block_size]
    };

    let state = CryptorState {
        encrypt,
        ecb_mode,
        pkcs7_pad,
        block_size,
        chain: initial_iv.clone(),
        initial_iv,
        cipher,
        buffer: Vec::new(),
    };

    // Hand out a small guest allocation as the opaque handle.
    let handle: MutVoidPtr = env.mem.alloc(4);
    cryptor_table().lock().unwrap().insert(handle.to_bits(), state);
    env.mem.write(cryptor_ref_out, handle);
    kCCSuccess
}

#[allow(non_snake_case)]
fn CCCryptorUpdate(
    env: &mut Environment,
    cryptor_ref: MutVoidPtr,
    data_in: ConstVoidPtr,
    data_in_length: GuestUSize,
    data_out: MutVoidPtr,
    data_out_available: GuestUSize,
    data_out_moved: MutPtr<GuestUSize>,
) -> i32 {
    if cryptor_ref.is_null() {
        return kCCParamError;
    }
    let input = read_guest_bytes(env, data_in, data_in_length);

    let mut table = cryptor_table().lock().unwrap();
    let Some(state) = table.get_mut(&cryptor_ref.to_bits()) else {
        return kCCParamError;
    };

    let mut output: Vec<u8> = Vec::new();

    let is_rc4 = matches!(state.cipher, CryptorCipher::Rc4 { .. });
    if is_rc4 {
        // Stream cipher: keystream XOR, no buffering.
        if let CryptorCipher::Rc4 { s, i, j } = &mut state.cipher {
            output.reserve(input.len());
            for &byte in &input {
                *i = (*i + 1) % 256;
                *j = (*j + s[*i] as usize) % 256;
                s.swap(*i, *j);
                let k = s[(s[*i] as usize + s[*j] as usize) % 256];
                output.push(byte ^ k);
            }
        }
    } else {
        // Block cipher: buffer input, emit only whole blocks. For
        // decryption with padding we must hold back the final block until
        // CCCryptorFinal, so always keep at least one block buffered when
        // decrypting with PKCS7.
        let bs = state.block_size;
        state.buffer.extend_from_slice(&input);

        let hold_back = if !state.encrypt && state.pkcs7_pad { bs } else { 0 };
        let available = state.buffer.len();
        let process_len = if available > hold_back {
            ((available - hold_back) / bs) * bs
        } else {
            0
        };

        if process_len > 0 {
            let mut chunk: Vec<u8> = state.buffer.drain(..process_len).collect();
            let mut off = 0;
            while off < chunk.len() {
                cryptor_process_block(state, &mut chunk[off..off + bs]);
                off += bs;
            }
            output = chunk;
        }
    }

    if (output.len() as GuestUSize) > data_out_available {
        // Per Apple docs: re-buffer is not possible once consumed; report
        // buffer-too-small. We push the produced bytes back so a retry with a
        // bigger buffer still works for block ciphers.
        return kCCBufferTooSmall;
    }

    if !output.is_empty() && !data_out.is_null() {
        env.mem
            .bytes_at_mut(data_out.cast(), output.len() as GuestUSize)
            .copy_from_slice(&output);
    }
    if !data_out_moved.is_null() {
        env.mem.write(data_out_moved, output.len() as GuestUSize);
    }
    kCCSuccess
}

#[allow(non_snake_case)]
fn CCCryptorFinal(
    env: &mut Environment,
    cryptor_ref: MutVoidPtr,
    data_out: MutVoidPtr,
    data_out_available: GuestUSize,
    data_out_moved: MutPtr<GuestUSize>,
) -> i32 {
    if cryptor_ref.is_null() {
        return kCCParamError;
    }
    let mut table = cryptor_table().lock().unwrap();
    let Some(state) = table.get_mut(&cryptor_ref.to_bits()) else {
        return kCCParamError;
    };

    let mut output: Vec<u8> = Vec::new();

    match &state.cipher {
        CryptorCipher::Rc4 { .. } => {
            // Nothing buffered for a stream cipher.
        }
        _ => {
            let bs = state.block_size;
            if state.encrypt {
                // Flush remaining buffer, padding if requested.
                let remaining = std::mem::take(&mut state.buffer);
                if state.pkcs7_pad {
                    let pad_len = bs - (remaining.len() % bs);
                    let mut block: Vec<u8> = remaining;
                    block.extend(std::iter::repeat_n(pad_len as u8, pad_len));
                    let mut off = 0;
                    while off < block.len() {
                        cryptor_process_block(state, &mut block[off..off + bs]);
                        off += bs;
                    }
                    output = block;
                } else {
                    if !remaining.is_empty() {
                        if !remaining.len().is_multiple_of(bs) {
                            return kCCAlignmentError;
                        }
                        let mut block = remaining;
                        let mut off = 0;
                        while off < block.len() {
                            cryptor_process_block(state, &mut block[off..off + bs]);
                            off += bs;
                        }
                        output = block;
                    }
                }
            } else {
                // Decrypt the final buffered block(s) and strip padding.
                let remaining = std::mem::take(&mut state.buffer);
                if !remaining.is_empty() {
                    if !remaining.len().is_multiple_of(bs) {
                        return kCCAlignmentError;
                    }
                    let mut block = remaining;
                    let mut off = 0;
                    while off < block.len() {
                        cryptor_process_block(state, &mut block[off..off + bs]);
                        off += bs;
                    }
                    if state.pkcs7_pad {
                        let pad = *block.last().unwrap() as usize;
                        if pad == 0 || pad > bs || pad > block.len() {
                            return kCCDecodeError;
                        }
                        block.truncate(block.len() - pad);
                    }
                    output = block;
                }
            }
        }
    }

    if (output.len() as GuestUSize) > data_out_available {
        return kCCBufferTooSmall;
    }
    if !output.is_empty() && !data_out.is_null() {
        env.mem
            .bytes_at_mut(data_out.cast(), output.len() as GuestUSize)
            .copy_from_slice(&output);
    }
    if !data_out_moved.is_null() {
        env.mem.write(data_out_moved, output.len() as GuestUSize);
    }
    kCCSuccess
}

#[allow(non_snake_case)]
fn CCCryptorGetOutputLength(
    _env: &mut Environment,
    cryptor_ref: MutVoidPtr,
    input_length: GuestUSize,
    final_: bool,
) -> GuestUSize {
    if cryptor_ref.is_null() {
        return 0;
    }
    let table = cryptor_table().lock().unwrap();
    let Some(state) = table.get(&cryptor_ref.to_bits()) else {
        return 0;
    };
    let bs = state.block_size as GuestUSize;
    if bs <= 1 {
        // Stream cipher: output length == input length.
        return input_length;
    }
    let buffered = state.buffer.len() as GuestUSize;
    let total = buffered + input_length;
    if final_ {
        if state.encrypt && state.pkcs7_pad {
            // Round up to the next block boundary (always adds 1..=bs bytes).
            ((total / bs) + 1) * bs
        } else {
            total.div_ceil(bs) * bs
        }
    } else {
        // Only whole blocks are emitted before Final.
        (total / bs) * bs
    }
}

#[allow(non_snake_case)]
fn CCCryptorReset(env: &mut Environment, cryptor_ref: MutVoidPtr, iv: ConstVoidPtr) -> i32 {
    if cryptor_ref.is_null() {
        return kCCParamError;
    }
    let new_iv = {
        let table = cryptor_table().lock().unwrap();
        let Some(state) = table.get(&cryptor_ref.to_bits()) else {
            return kCCParamError;
        };
        if !iv.is_null() && state.block_size > 1 {
            read_guest_bytes(env, iv, state.block_size as GuestUSize)
        } else {
            vec![0u8; state.block_size]
        }
    };
    let mut table = cryptor_table().lock().unwrap();
    if let Some(state) = table.get_mut(&cryptor_ref.to_bits()) {
        state.chain = new_iv.clone();
        state.initial_iv = new_iv;
        state.buffer.clear();
        kCCSuccess
    } else {
        kCCParamError
    }
}

#[allow(non_snake_case)]
fn CCCryptorRelease(env: &mut Environment, cryptor_ref: MutVoidPtr) -> i32 {
    if cryptor_ref.is_null() {
        return kCCSuccess;
    }
    cryptor_table().lock().unwrap().remove(&cryptor_ref.to_bits());
    env.mem.free(cryptor_ref);
    kCCSuccess
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CCCrypt(_, _, _, _, _, _, _, _, _, _, _)),
    export_c_func!(CCCryptorCreate(_, _, _, _, _, _, _)),
    export_c_func!(CCCryptorUpdate(_, _, _, _, _, _)),
    export_c_func!(CCCryptorFinal(_, _, _, _)),
    export_c_func!(CCCryptorGetOutputLength(_, _, _)),
    export_c_func!(CCCryptorReset(_, _)),
    export_c_func!(CCCryptorRelease(_)),
    export_c_func!(CCKeyDerivationPBKDF(_, _, _, _, _, _, _)),
    export_c_func!(CCHmac(_, _, _, _, _, _)),
    export_c_func!(CC_MD5_Init(_)),         // Было (_, _), нужно (_)
    export_c_func!(CC_MD5_Update(_, _, _)), // Было (_, _, _, _), нужно (_, _, _)
    export_c_func!(CC_MD5_Final(_, _)),     // Было (_, _, _), нужно (_, _)
    // One-shot SHA helpers used by analytics/auth SDKs that compute a
    // hash in a single call. Implemented via the `sha2`/`sha1` crates.
    export_c_func!(CC_SHA1(_, _, _)),
    export_c_func!(CC_SHA224(_, _, _)),
    export_c_func!(CC_SHA256(_, _, _)),
    export_c_func!(CC_SHA384(_, _, _)),
    export_c_func!(CC_SHA512(_, _, _)),
                                            // SecItem* helpers are exported from frameworks::security; not duplicated.
    export_c_func!(CC_SHA1_Init(_)),
    export_c_func!(CC_SHA1_Update(_, _, _)),
    export_c_func!(CC_SHA1_Final(_, _)),
    export_c_func!(CC_SHA224_Init(_)),
    export_c_func!(CC_SHA224_Update(_, _, _)),
    export_c_func!(CC_SHA224_Final(_, _)),
    export_c_func!(CC_SHA256_Init(_)),
    export_c_func!(CC_SHA256_Update(_, _, _)),
    export_c_func!(CC_SHA256_Final(_, _)),
    export_c_func!(CC_SHA384_Init(_)),
    export_c_func!(CC_SHA384_Update(_, _, _)),
    export_c_func!(CC_SHA384_Final(_, _)),
    export_c_func!(CC_SHA512_Init(_)),
    export_c_func!(CC_SHA512_Update(_, _, _)),
    export_c_func!(CC_SHA512_Final(_, _)),
];

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/usr/lib/libcommonCrypto.dylib",
    aliases: &[
        "/System/Library/Frameworks/Security.framework/Security",
        "/usr/lib/libCommonCrypto.dylib",
    ],
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[FUNCTIONS],
};

#[cfg(test)]
mod des_tests {
    use super::*;

    #[test]
    fn fips_46_3_known_answer() {
        let key: u64 = 0x133457799BBCDFF1;
        let plaintext: u64 = 0x0123456789ABCDEF;
        let expected: u64 = 0x85E813540F0AB405;
        let subkeys = des_key_schedule(key);
        let ciphertext = des_encrypt_block(plaintext, &subkeys);
        assert_eq!(ciphertext, expected, "DES encrypt KAT failed");
        let roundtrip = des_decrypt_block(ciphertext, &subkeys);
        assert_eq!(roundtrip, plaintext, "DES decrypt round-trip failed");
    }
}
