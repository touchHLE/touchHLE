use crate::dyld::{export_c_func, FunctionExports, HostDylib};
use crate::mem::{ConstPtr, MutPtr};
use crate::Environment;
use bzip2::Decompress;
use std::collections::HashMap;
use std::sync::Mutex;

const BZ_OK: i32 = 0;
const BZ_RUN_OK: i32 = 1;
const BZ_FLUSH_OK: i32 = 2;
const BZ_FINISH_OK: i32 = 3;
const BZ_STREAM_END: i32 = 4;
const BZ_PARAM_ERROR: i32 = -2;
const BZ_DATA_ERROR: i32 = -4;

lazy_static::lazy_static! {
    static ref DECOMPRESSORS: Mutex<HashMap<u32, Decompress>> = Mutex::new(HashMap::new());
}

#[allow(non_snake_case)]
pub fn BZ2_bzDecompressInit(
    _env: &mut Environment,
    strm_ptr: u32,
    _verbosity: i32,
    small: i32,
) -> i32 {
    if strm_ptr == 0 {
        return BZ_PARAM_ERROR;
    }

    let mut map = DECOMPRESSORS.lock().unwrap();
    map.insert(strm_ptr, Decompress::new(small != 0));

    BZ_OK
}

#[allow(non_snake_case)]
pub fn BZ2_bzDecompress(env: &mut Environment, strm_ptr: u32) -> i32 {
    if strm_ptr == 0 {
        return BZ_PARAM_ERROR;
    }

    let mut map = DECOMPRESSORS.lock().unwrap();
    let decompressor = match map.get_mut(&strm_ptr) {
        Some(d) => d,
        None => return BZ_PARAM_ERROR,
    };

    let next_in_ptr: u32 = env.mem.read(ConstPtr::<u32>::from_bits(strm_ptr));
    let avail_in: u32 = env.mem.read(ConstPtr::<u32>::from_bits(strm_ptr + 4));
    let next_out_ptr: u32 = env.mem.read(ConstPtr::<u32>::from_bits(strm_ptr + 16));
    let avail_out: u32 = env.mem.read(ConstPtr::<u32>::from_bits(strm_ptr + 20));

    // Копируем входные данные в вектор, чтобы сразу отпустить `env.mem`
    let in_buf = env
        .mem
        .bytes_at(ConstPtr::<u8>::from_bits(next_in_ptr), avail_in)
        .to_vec();

    // Теперь мы можем свободно взять `out_buf` как изменяемый
    let out_buf = env
        .mem
        .bytes_at_mut(MutPtr::<u8>::from_bits(next_out_ptr), avail_out);

    let before_in = decompressor.total_in();
    let before_out = decompressor.total_out();

    // Передаем ссылку на наш вектор &in_buf
    let status = match decompressor.decompress(&in_buf, out_buf) {
        Ok(bzip2::Status::Ok) => BZ_OK,
        Ok(bzip2::Status::RunOk) => BZ_RUN_OK,
        Ok(bzip2::Status::FlushOk) => BZ_FLUSH_OK,
        Ok(bzip2::Status::FinishOk) => BZ_FINISH_OK,
        Ok(bzip2::Status::StreamEnd) => BZ_STREAM_END,
        Err(_) => return BZ_DATA_ERROR,
        _ => BZ_OK,
    };

    let consumed_in = (decompressor.total_in() - before_in) as u32;
    let produced_out = (decompressor.total_out() - before_out) as u32;

    env.mem.write(
        MutPtr::<u32>::from_bits(strm_ptr),
        next_in_ptr + consumed_in,
    );
    env.mem.write(
        MutPtr::<u32>::from_bits(strm_ptr + 4),
        avail_in - consumed_in,
    );
    env.mem.write(
        MutPtr::<u32>::from_bits(strm_ptr + 16),
        next_out_ptr + produced_out,
    );
    env.mem.write(
        MutPtr::<u32>::from_bits(strm_ptr + 20),
        avail_out - produced_out,
    );

    let total_in_lo: u32 = env.mem.read(ConstPtr::<u32>::from_bits(strm_ptr + 8));
    env.mem.write(
        MutPtr::<u32>::from_bits(strm_ptr + 8),
        total_in_lo.wrapping_add(consumed_in),
    );

    let total_out_lo: u32 = env.mem.read(ConstPtr::<u32>::from_bits(strm_ptr + 24));
    env.mem.write(
        MutPtr::<u32>::from_bits(strm_ptr + 24),
        total_out_lo.wrapping_add(produced_out),
    );

    status
}

#[allow(non_snake_case)]
pub fn BZ2_bzDecompressEnd(_env: &mut Environment, strm_ptr: u32) -> i32 {
    let mut map = DECOMPRESSORS.lock().unwrap();
    if map.remove(&strm_ptr).is_some() {
        BZ_OK
    } else {
        BZ_PARAM_ERROR
    }
}

#[allow(non_snake_case)]
pub fn BZ2_bzBuffToBuffDecompress(
    env: &mut Environment,
    dest: u32,
    dest_len_ptr: u32,
    source: u32,
    source_len: u32,
    small: i32,
    _verbosity: i32,
) -> i32 {
    let dest_len: u32 = env.mem.read(ConstPtr::<u32>::from_bits(dest_len_ptr));

    // Добавляем .to_vec()
    let in_buf = env
        .mem
        .bytes_at(ConstPtr::<u8>::from_bits(source), source_len)
        .to_vec();
    let out_buf = env
        .mem
        .bytes_at_mut(MutPtr::<u8>::from_bits(dest), dest_len);

    let mut decompressor = Decompress::new(small != 0);
    // Передаем ссылку &in_buf
    match decompressor.decompress(&in_buf, out_buf) {
        Ok(bzip2::Status::StreamEnd) | Ok(bzip2::Status::Ok) => {
            let produced = decompressor.total_out() as u32;
            env.mem
                .write(MutPtr::<u32>::from_bits(dest_len_ptr), produced);
            BZ_OK
        }
        _ => BZ_DATA_ERROR,
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(BZ2_bzDecompressInit(_, _, _)),
    export_c_func!(BZ2_bzDecompress(_)),
    export_c_func!(BZ2_bzDecompressEnd(_)),
    export_c_func!(BZ2_bzBuffToBuffDecompress(_, _, _, _, _, _)),
];

// Добавляем объявление DYLIB прямо сюда, как это принято в других модулях
// touchHLE
pub const DYLIB: HostDylib = HostDylib {
    path: "/usr/lib/libbz2.1.0.dylib",
    aliases: &["/usr/lib/libbz2.dylib"], // На всякий случай добавим алиас
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[FUNCTIONS],
};
