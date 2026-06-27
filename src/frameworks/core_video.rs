use crate::dyld::{ConstantExports, HostConstant, HostDylib, HostFunction};
use crate::mem::{ConstPtr, GuestUSize, MutPtr, MutVoidPtr};
use crate::Environment;
use std::sync::Mutex;

// CoreVideo NSString-typed constants used by capture / pixel-buffer APIs.
// Exposed as Mach-O symbols (e.g. `extern NSString * const kCVPixelBufferPixelFormatTypeKey`).
pub const kCVPixelBufferPixelFormatTypeKey: &str = "PixelFormatType";
pub const kCVPixelBufferWidthKey: &str = "Width";
pub const kCVPixelBufferHeightKey: &str = "Height";
pub const kCVPixelBufferBytesPerRowAlignmentKey: &str = "BytesPerRowAlignment";
pub const kCVPixelBufferCGImageCompatibilityKey: &str = "CGImageCompatibility";
pub const kCVPixelBufferCGBitmapContextCompatibilityKey: &str = "CGBitmapContextCompatibility";
pub const kCVPixelBufferOpenGLESCompatibilityKey: &str = "OpenGLESCompatibility";
pub const kCVPixelBufferIOSurfacePropertiesKey: &str = "IOSurfaceProperties";
pub const kCVPixelBufferPlaneAlignmentKey: &str = "PlaneAlignment";
pub const kCVPixelBufferExtendedPixelsLeftKey: &str = "ExtendedPixelsLeft";
pub const kCVPixelBufferExtendedPixelsTopKey: &str = "ExtendedPixelsTop";
pub const kCVPixelBufferExtendedPixelsRightKey: &str = "ExtendedPixelsRight";
pub const kCVPixelBufferExtendedPixelsBottomKey: &str = "ExtendedPixelsBottom";
pub const kCVPixelBufferMemoryAllocatorKey: &str = "MemoryAllocator";

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCVPixelBufferPixelFormatTypeKey",
        HostConstant::NSString(kCVPixelBufferPixelFormatTypeKey),
    ),
    (
        "_kCVPixelBufferWidthKey",
        HostConstant::NSString(kCVPixelBufferWidthKey),
    ),
    (
        "_kCVPixelBufferHeightKey",
        HostConstant::NSString(kCVPixelBufferHeightKey),
    ),
    (
        "_kCVPixelBufferBytesPerRowAlignmentKey",
        HostConstant::NSString(kCVPixelBufferBytesPerRowAlignmentKey),
    ),
    (
        "_kCVPixelBufferCGImageCompatibilityKey",
        HostConstant::NSString(kCVPixelBufferCGImageCompatibilityKey),
    ),
    (
        "_kCVPixelBufferCGBitmapContextCompatibilityKey",
        HostConstant::NSString(kCVPixelBufferCGBitmapContextCompatibilityKey),
    ),
    (
        "_kCVPixelBufferOpenGLESCompatibilityKey",
        HostConstant::NSString(kCVPixelBufferOpenGLESCompatibilityKey),
    ),
    (
        "_kCVPixelBufferIOSurfacePropertiesKey",
        HostConstant::NSString(kCVPixelBufferIOSurfacePropertiesKey),
    ),
    (
        "_kCVPixelBufferPlaneAlignmentKey",
        HostConstant::NSString(kCVPixelBufferPlaneAlignmentKey),
    ),
    (
        "_kCVPixelBufferExtendedPixelsLeftKey",
        HostConstant::NSString(kCVPixelBufferExtendedPixelsLeftKey),
    ),
    (
        "_kCVPixelBufferExtendedPixelsTopKey",
        HostConstant::NSString(kCVPixelBufferExtendedPixelsTopKey),
    ),
    (
        "_kCVPixelBufferExtendedPixelsRightKey",
        HostConstant::NSString(kCVPixelBufferExtendedPixelsRightKey),
    ),
    (
        "_kCVPixelBufferExtendedPixelsBottomKey",
        HostConstant::NSString(kCVPixelBufferExtendedPixelsBottomKey),
    ),
    // Apple `CVPixelBuffer.h` — `kCVPixelBufferMemoryAllocatorKey` is a
    // CFStringRef key for a `CFAllocatorRef` value. Apps thread their
    // own allocator through CVPixelBufferCreate; touchHLE's CVPixelBuffer
    // implementation always allocates via the default allocator, so the
    // key participates only in dictionary-equality comparisons.
    (
        "_kCVPixelBufferMemoryAllocatorKey",
        HostConstant::NSString(kCVPixelBufferMemoryAllocatorKey),
    ),
    // -----------------------------------------------------------------
    // `kCVPixelBufferMetalCompatibilityKey` (iOS 8+). Apple
    // `CVPixelBuffer.h` declares it as `extern const CFStringRef`. The
    // literal value of the published key is `MetalCompatibility`; apps
    // use it as a `pixelBufferAttributes` dictionary key, so the only
    // requirement is that it remains a unique CFStringRef.
    // <https://developer.apple.com/documentation/corevideo/kcvpixelbuffermetalcompatibilitykey>
    // -----------------------------------------------------------------
    (
        "_kCVPixelBufferMetalCompatibilityKey",
        HostConstant::NSString("MetalCompatibility"),
    ),
    // -----------------------------------------------------------------
    // CVImageBuffer attachment / matrix keys. Apple `CVImageBuffer.h`.
    // Apps store and read them via
    // `CVBufferSetAttachment` / `CVBufferGetAttachment`.
    // <https://developer.apple.com/documentation/corevideo/kcvimagebufferycbcrmatrixkey>
    // -----------------------------------------------------------------
    (
        "_kCVImageBufferYCbCrMatrixKey",
        HostConstant::NSString("YCbCrMatrix"),
    ),
    (
        "_kCVImageBufferYCbCrMatrix_ITU_R_601_4",
        HostConstant::NSString("ITU_R_601_4"),
    ),
    (
        "_kCVImageBufferYCbCrMatrix_ITU_R_709_2",
        HostConstant::NSString("ITU_R_709_2"),
    ),
    (
        "_kCVImageBufferYCbCrMatrix_SMPTE_240M_1995",
        HostConstant::NSString("SMPTE_240M_1995"),
    ),
    (
        "_kCVImageBufferColorPrimariesKey",
        HostConstant::NSString("ColorPrimaries"),
    ),
    (
        "_kCVImageBufferTransferFunctionKey",
        HostConstant::NSString("TransferFunction"),
    ),
];

// Структура для управления pixel buffer в памяти гостя
struct PixelBufferInfo {
    guest_ptr: u32,
    width: u32,
    height: u32,
    bytes_per_row: u32,
    pixel_format: u32,
    plane_count: u32,
    is_locked: bool,
}

// Глобальное хранилище для pixel buffers
static PIXEL_BUFFERS: Mutex<Option<PixelBufferInfo>> = Mutex::new(None);

// Константы форматов пикселей
const K_CV_PIXEL_FORMAT_TYPE_32BGRA: u32 = 0x42475241; // 'BGRA'
const K_CV_PIXEL_FORMAT_TYPE_32ARGB: u32 = 0x00000020; // 32-bit ARGB
const K_CV_PIXEL_FORMAT_TYPE_420YpCbCr8BiPlanarFullRange: u32 = 0x34323066; // '420f'
const K_CV_PIXEL_FORMAT_TYPE_420YpCbCr8BiPlanarVideoRange: u32 = 0x34323076; // '420v'

// Константы для lock flags
const K_CV_PIXEL_BUFFER_LOCK_READ_ONLY: i32 = 0x00000001;
const K_CV_PIXEL_BUFFER_LOCK_WRITE_ONLY: i32 = 0x00000002;

// Код возврата успеха
const K_CV_RETURN_SUCCESS: i32 = 0;
const K_CV_RETURN_ERROR: i32 = -6660;
const K_CV_RETURN_INVALID_ARGUMENT: i32 = -6661;

// ===== ОСНОВНЫЕ ФУНКЦИИ PIXEL BUFFER =====

pub fn CVPixelBufferGetWidth(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> i32 {
    let buffers = PIXEL_BUFFERS.lock().unwrap();
    if let Some(ref info) = *buffers {
        info.width as i32
    } else {
        640
    }
}

pub fn CVPixelBufferGetHeight(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> i32 {
    let buffers = PIXEL_BUFFERS.lock().unwrap();
    if let Some(ref info) = *buffers {
        info.height as i32
    } else {
        480
    }
}

pub fn CVPixelBufferGetBaseAddress(env: &mut Environment, _pixel_buffer: MutVoidPtr) -> u32 {
    let mut buffers = PIXEL_BUFFERS.lock().unwrap();
    if let Some(ref info) = *buffers {
        return info.guest_ptr;
    }

    // Создаем новый буфер
    let width = 640u32;
    let height = 480u32;
    let bytes_per_pixel = 4u32; // BGRA
    let bytes_per_row = width * bytes_per_pixel;
    let size: GuestUSize = (bytes_per_row * height) as GuestUSize;

    // Выделяем память в гостевой системе
    let ptr = env.mem.alloc(size);
    // Безопасное преобразование указателя в u32
    // Используем .cast().to_bits() для получения сырого значения адреса
    let guest_ptr = ptr.cast::<u8>().to_bits();

    // Инициализируем буфер черным цветом (опционально)
    for offset in 0..size {
        env.mem.write(ptr.cast::<u8>() + offset, 0u8);
    }

    *buffers = Some(PixelBufferInfo {
        guest_ptr,
        width,
        height,
        bytes_per_row,
        pixel_format: K_CV_PIXEL_FORMAT_TYPE_32BGRA,
        plane_count: 1,
        is_locked: false,
    });

    guest_ptr
}

pub fn CVPixelBufferLockBaseAddress(
    _env: &mut Environment,
    _pixel_buffer: MutVoidPtr,
    _lock_flags: i32,
) -> i32 {
    let mut buffers = PIXEL_BUFFERS.lock().unwrap();
    if let Some(ref mut info) = *buffers {
        if info.is_locked {
            return K_CV_RETURN_ERROR;
        }
        info.is_locked = true;
        K_CV_RETURN_SUCCESS
    } else {
        K_CV_RETURN_INVALID_ARGUMENT
    }
}

pub fn CVPixelBufferUnlockBaseAddress(
    _env: &mut Environment,
    _pixel_buffer: MutVoidPtr,
    _unlock_flags: i32,
) -> i32 {
    let mut buffers = PIXEL_BUFFERS.lock().unwrap();
    if let Some(ref mut info) = *buffers {
        if !info.is_locked {
            return K_CV_RETURN_ERROR;
        }
        info.is_locked = false;
        K_CV_RETURN_SUCCESS
    } else {
        K_CV_RETURN_INVALID_ARGUMENT
    }
}

pub fn CVPixelBufferGetPixelFormatType(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> u32 {
    let buffers = PIXEL_BUFFERS.lock().unwrap();
    if let Some(ref info) = *buffers {
        info.pixel_format
    } else {
        K_CV_PIXEL_FORMAT_TYPE_32BGRA
    }
}

pub fn CVPixelBufferGetBytesPerRow(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> u32 {
    let buffers = PIXEL_BUFFERS.lock().unwrap();
    if let Some(ref info) = *buffers {
        info.bytes_per_row
    } else {
        640 * 4
    }
}

pub fn CVPixelBufferGetBytesPerRowOfPlane(
    _env: &mut Environment,
    _pixel_buffer: MutVoidPtr,
    plane_index: u32,
) -> u32 {
    let buffers = PIXEL_BUFFERS.lock().unwrap();
    if let Some(ref info) = *buffers {
        if plane_index < info.plane_count {
            info.bytes_per_row
        } else {
            0
        }
    } else {
        if plane_index == 0 {
            640 * 4
        } else {
            0
        }
    }
}

pub fn CVPixelBufferGetPlaneCount(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> u32 {
    let buffers = PIXEL_BUFFERS.lock().unwrap();
    if let Some(ref info) = *buffers {
        info.plane_count
    } else {
        1
    }
}

pub fn CVPixelBufferGetBaseAddressOfPlane(
    env: &mut Environment,
    pixel_buffer: MutVoidPtr,
    plane_index: u32,
) -> u32 {
    if plane_index == 0 {
        CVPixelBufferGetBaseAddress(env, pixel_buffer)
    } else {
        // Для многоплоскостных форматов (например, YUV)
        // возвращаем смещение для следующих плоскостей
        let buffers = PIXEL_BUFFERS.lock().unwrap();
        if let Some(ref info) = *buffers {
            if plane_index < info.plane_count {
                // Для простоты возвращаем тот же адрес
                // В реальной реализации здесь было бы смещение
                info.guest_ptr
            } else {
                0
            }
        } else {
            0
        }
    }
}

pub fn CVPixelBufferGetWidthOfPlane(
    env: &mut Environment,
    pixel_buffer: MutVoidPtr,
    plane_index: u32,
) -> u32 {
    if plane_index == 0 {
        CVPixelBufferGetWidth(env, pixel_buffer) as u32
    } else {
        // Для UV плоскости в YUV формате ширина может быть половинной
        let buffers = PIXEL_BUFFERS.lock().unwrap();
        if let Some(ref info) = *buffers {
            if plane_index < info.plane_count {
                info.width / 2
            } else {
                0
            }
        } else {
            0
        }
    }
}

pub fn CVPixelBufferGetHeightOfPlane(
    env: &mut Environment,
    pixel_buffer: MutVoidPtr,
    plane_index: u32,
) -> u32 {
    if plane_index == 0 {
        CVPixelBufferGetHeight(env, pixel_buffer) as u32
    } else {
        let buffers = PIXEL_BUFFERS.lock().unwrap();
        if let Some(ref info) = *buffers {
            if plane_index < info.plane_count {
                info.height / 2
            } else {
                0
            }
        } else {
            0
        }
    }
}

pub fn CVPixelBufferIsPlanar(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> u32 {
    let buffers = PIXEL_BUFFERS.lock().unwrap();
    if let Some(ref info) = *buffers {
        if info.plane_count > 1 {
            1
        } else {
            0
        }
    } else {
        0
    }
}

pub fn CVPixelBufferRelease(_env: &mut Environment, _pixel_buffer: MutVoidPtr) {
    // В реальной реализации здесь был бы reference counting
    // Для простоты не очищаем буфер, так как может использоваться
}

pub fn CVPixelBufferRetain(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> MutVoidPtr {
    // В реальной реализации здесь был бы reference counting
    _pixel_buffer
}

// ===== ФУНКЦИИ SAMPLE BUFFER =====

pub fn CMSampleBufferGetImageBuffer(_env: &mut Environment, _sample_buffer: MutVoidPtr) -> u32 {
    // Возвращаем фейковый не-нулевой указатель на pixel buffer
    1
}

pub fn CMSampleBufferGetNumSamples(_env: &mut Environment, _sample_buffer: MutVoidPtr) -> i32 {
    1
}

pub fn CMSampleBufferDataIsReady(_env: &mut Environment, _sample_buffer: MutVoidPtr) -> u32 {
    1 // true - данные всегда готовы
}

pub fn CMSampleBufferIsValid(_env: &mut Environment, _sample_buffer: MutVoidPtr) -> u32 {
    1 // true
}

pub fn CMSampleBufferInvalidate(_env: &mut Environment, _sample_buffer: MutVoidPtr) {
    // Ничего не делаем
}

// ===== ДОПОЛНИТЕЛЬНЫЕ ПОЛЕЗНЫЕ ФУНКЦИИ =====

pub fn CVPixelBufferGetDataSize(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> u32 {
    let buffers = PIXEL_BUFFERS.lock().unwrap();
    if let Some(ref info) = *buffers {
        info.bytes_per_row * info.height
    } else {
        640 * 480 * 4
    }
}

pub fn CVPixelBufferGetExtendedPixels(
    _env: &mut Environment,
    _pixel_buffer: MutVoidPtr,
    _extra_columns_on_left: MutPtr<u32>,
    _extra_columns_on_right: MutPtr<u32>,
    _extra_rows_on_top: MutPtr<u32>,
    _extra_rows_on_bottom: MutPtr<u32>,
) {
    // Для простоты возвращаем нули - нет дополнительных пикселей
    // В реальности эти значения могут быть ненулевыми для некоторых форматов
}

pub fn CVPixelBufferFillExtendedPixels(_env: &mut Environment, _pixel_buffer: MutVoidPtr) -> i32 {
    K_CV_RETURN_SUCCESS
}

pub fn CVBufferGetAttachment(
    _env: &mut Environment,
    _buffer: MutVoidPtr,
    _key: ConstPtr<u8>,
    _attachment_mode: MutPtr<u32>,
) -> u32 {
    0 // NULL - нет attachments
}

pub fn CVBufferSetAttachment(
    _env: &mut Environment,
    _buffer: MutVoidPtr,
    _key: ConstPtr<u8>,
    _value: MutVoidPtr,
    _attachment_mode: u32,
) {
    // Ничего не делаем
}

pub fn CVBufferRemoveAttachment(_env: &mut Environment, _buffer: MutVoidPtr, _key: ConstPtr<u8>) {
    // Ничего не делаем
}

pub fn CVBufferRemoveAllAttachments(_env: &mut Environment, _buffer: MutVoidPtr) {
    // Ничего не делаем
}

// ===== ВСПОМОГАТЕЛЬНАЯ ФУНКЦИЯ ДЛЯ УСТАНОВКИ ПАРАМЕТРОВ БУФЕРА =====

pub fn CVPixelBufferCreate(
    env: &mut Environment,
    _allocator: MutVoidPtr,
    width: u32,
    height: u32,
    pixel_format_type: u32,
    _pixel_buffer_attributes: MutVoidPtr,
    pixel_buffer_out: MutPtr<u32>,
) -> i32 {
    let bytes_per_pixel = match pixel_format_type {
        K_CV_PIXEL_FORMAT_TYPE_32BGRA | K_CV_PIXEL_FORMAT_TYPE_32ARGB => 4,
        K_CV_PIXEL_FORMAT_TYPE_420YpCbCr8BiPlanarFullRange
        | K_CV_PIXEL_FORMAT_TYPE_420YpCbCr8BiPlanarVideoRange => 1, // Y plane
        _ => 4, // По умолчанию
    };

    let bytes_per_row = width * bytes_per_pixel;
    let size: GuestUSize = (bytes_per_row * height) as GuestUSize;

    let ptr = env.mem.alloc(size);
    let guest_ptr = ptr.cast::<u8>().to_bits();

    // Инициализируем буфер
    for offset in 0..size {
        env.mem.write(ptr.cast::<u8>() + offset, 0u8);
    }

    let mut buffers = PIXEL_BUFFERS.lock().unwrap();
    *buffers = Some(PixelBufferInfo {
        guest_ptr,
        width,
        height,
        bytes_per_row,
        pixel_format: pixel_format_type,
        plane_count: if pixel_format_type == K_CV_PIXEL_FORMAT_TYPE_32BGRA {
            1
        } else {
            2
        },
        is_locked: false,
    });

    // Записываем указатель в выходной параметр
    env.mem.write(pixel_buffer_out, guest_ptr);
    K_CV_RETURN_SUCCESS
}

// ===== РЕГИСТРАЦИЯ ВСЕХ ФУНКЦИЙ ДЛЯ ЭМУЛЯТОРА =====

pub const FUNCTIONS: &[(&str, HostFunction)] = &[
    // Основные функции pixel buffer
    (
        "_CVPixelBufferGetWidth",
        &(CVPixelBufferGetWidth as fn(&mut Environment, _) -> _),
    ),
    (
        "_CVPixelBufferGetHeight",
        &(CVPixelBufferGetHeight as fn(&mut Environment, _) -> _),
    ),
    (
        "_CVPixelBufferGetBaseAddress",
        &(CVPixelBufferGetBaseAddress as fn(&mut Environment, _) -> _),
    ),
    (
        "_CVPixelBufferLockBaseAddress",
        &(CVPixelBufferLockBaseAddress as fn(&mut Environment, _, _) -> _),
    ),
    (
        "_CVPixelBufferUnlockBaseAddress",
        &(CVPixelBufferUnlockBaseAddress as fn(&mut Environment, _, _) -> _),
    ),
    (
        "_CVPixelBufferGetPixelFormatType",
        &(CVPixelBufferGetPixelFormatType as fn(&mut Environment, _) -> _),
    ),
    (
        "_CVPixelBufferGetBytesPerRow",
        &(CVPixelBufferGetBytesPerRow as fn(&mut Environment, _) -> _),
    ),
    // Функции для работы с плоскостями
    (
        "_CVPixelBufferGetBytesPerRowOfPlane",
        &(CVPixelBufferGetBytesPerRowOfPlane as fn(&mut Environment, _, _) -> _),
    ),
    (
        "_CVPixelBufferGetPlaneCount",
        &(CVPixelBufferGetPlaneCount as fn(&mut Environment, _) -> _),
    ),
    (
        "_CVPixelBufferGetBaseAddressOfPlane",
        &(CVPixelBufferGetBaseAddressOfPlane as fn(&mut Environment, _, _) -> _),
    ),
    (
        "_CVPixelBufferGetWidthOfPlane",
        &(CVPixelBufferGetWidthOfPlane as fn(&mut Environment, _, _) -> _),
    ),
    (
        "_CVPixelBufferGetHeightOfPlane",
        &(CVPixelBufferGetHeightOfPlane as fn(&mut Environment, _, _) -> _),
    ),
    (
        "_CVPixelBufferIsPlanar",
        &(CVPixelBufferIsPlanar as fn(&mut Environment, _) -> _),
    ),
    // Reference counting
    (
        "_CVPixelBufferRelease",
        &(CVPixelBufferRelease as fn(&mut Environment, _) -> _),
    ),
    (
        "_CVPixelBufferRetain",
        &(CVPixelBufferRetain as fn(&mut Environment, _) -> _),
    ),
    // Sample buffer функции
    (
        "_CMSampleBufferGetImageBuffer",
        &(CMSampleBufferGetImageBuffer as fn(&mut Environment, _) -> _),
    ),
    (
        "_CMSampleBufferGetNumSamples",
        &(CMSampleBufferGetNumSamples as fn(&mut Environment, _) -> _),
    ),
    (
        "_CMSampleBufferDataIsReady",
        &(CMSampleBufferDataIsReady as fn(&mut Environment, _) -> _),
    ),
    (
        "_CMSampleBufferIsValid",
        &(CMSampleBufferIsValid as fn(&mut Environment, _) -> _),
    ),
    (
        "_CMSampleBufferInvalidate",
        &(CMSampleBufferInvalidate as fn(&mut Environment, _) -> _),
    ),
    // Дополнительные функции
    (
        "_CVPixelBufferGetDataSize",
        &(CVPixelBufferGetDataSize as fn(&mut Environment, _) -> _),
    ),
    (
        "_CVPixelBufferFillExtendedPixels",
        &(CVPixelBufferFillExtendedPixels as fn(&mut Environment, _) -> _),
    ),
    (
        "_CVPixelBufferCreate",
        &(CVPixelBufferCreate as fn(&mut Environment, _, _, _, _, _, _) -> _),
    ),
    // Buffer attachments
    (
        "_CVBufferGetAttachment",
        &(CVBufferGetAttachment as fn(&mut Environment, _, _, _) -> _),
    ),
    (
        "_CVBufferSetAttachment",
        &(CVBufferSetAttachment as fn(&mut Environment, _, _, _, _) -> _),
    ),
    (
        "_CVBufferRemoveAttachment",
        &(CVBufferRemoveAttachment as fn(&mut Environment, _, _) -> _),
    ),
    (
        "_CVBufferRemoveAllAttachments",
        &(CVBufferRemoveAllAttachments as fn(&mut Environment, _) -> _),
    ),
];

// ===== РЕГИСТРАЦИЯ ФРЕЙМВОРКА =====

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/CoreVideo.framework/CoreVideo",
    aliases: &[],
    function_exports: &[FUNCTIONS],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
};
