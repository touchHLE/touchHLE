use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::libc::errno::set_errno;
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeRead};

#[repr(C, packed)]
#[allow(non_camel_case_types)]
pub struct malloc_zone_t {
    reserved1: MutVoidPtr,
    reserved2: MutVoidPtr,
    size: MutVoidPtr,
    malloc: MutVoidPtr,
    calloc: MutVoidPtr,
    valloc: MutVoidPtr,
    free: MutVoidPtr,
    realloc: MutVoidPtr,
    destroy: MutVoidPtr,
    zone_name: ConstPtr<u8>,
    batch_malloc: MutVoidPtr,
    batch_free: MutVoidPtr,
    introspect: MutVoidPtr,
    version: u32,
    memalign: MutVoidPtr,
}
unsafe impl SafeRead for malloc_zone_t {}

impl malloc_zone_t {
    pub fn new(env: &mut Environment) -> Self {
        Self {
            reserved1: Ptr::null(),
            reserved2: Ptr::null(),
            size: Self::get_func_address(env, "malloc_zone_size"),
            malloc: Self::get_func_address(env, "malloc_zone_malloc"),
            calloc: Ptr::null(),
            valloc: Ptr::null(),
            free: Self::get_func_address(env, "malloc_zone_free"),
            realloc: Self::get_func_address(env, "malloc_zone_realloc"),
            destroy: Self::get_func_address(env, "malloc_destroy_zone"),
            zone_name: Ptr::null(),
            batch_malloc: Ptr::null(),
            batch_free: Ptr::null(),
            introspect: Ptr::null(),
            version: 0,
            memalign: Ptr::null(),
        }
    }
    fn get_func_address(env: &mut Environment, func_name: &str) -> MutVoidPtr {
        let mangled_func_name = format!("_{func_name}");
        if let Ok(ptr) =
            env.dyld
                .create_proc_address(&mut env.mem, &mut env.cpu, &mangled_func_name)
        {
            MutVoidPtr::from_bits(ptr.addr_with_thumb_bit())
        } else {
            panic!("Request for procedure address of unimplemented function: {mangled_func_name}");
        }
    }
}

fn malloc_default_zone(env: &mut Environment) -> MutPtr<malloc_zone_t> {
    let zone = malloc_zone_t::new(env);
    env.mem.create_default_zone(zone)
}

fn malloc_zone_malloc(
    env: &mut Environment,
    zone: MutPtr<malloc_zone_t>,
    size: GuestUSize,
) -> MutVoidPtr {
    env.mem.zone_alloc(zone, size)
}

// This is not an actual system API, however
// we need to be able to provide it to the Guest
fn malloc_zone_size(
    env: &mut Environment,
    zone: MutPtr<malloc_zone_t>,
    ptr: ConstVoidPtr,
) -> GuestUSize {
    env.mem.malloc_zone_size(zone, ptr)
}

fn malloc_zone_realloc(
    env: &mut Environment,
    zone: MutPtr<malloc_zone_t>,
    ptr: MutVoidPtr,
    size: GuestUSize,
) -> MutVoidPtr {
    // TODO: handle errno properly
    set_errno(env, 0);

    if ptr.is_null() {
        return malloc_zone_malloc(env, zone, size);
    }
    env.mem.zone_realloc(zone, ptr, size)
}

fn malloc_zone_free(env: &mut Environment, zone: MutPtr<malloc_zone_t>, ptr: MutVoidPtr) {
    if ptr.is_null() {
        // "If ptr is a NULL pointer, no operation is performed."
        return;
    }
    env.mem.zone_free(zone, ptr);
}

fn malloc_create_zone(
    env: &mut Environment,
    _start_size: GuestUSize,
    _flags: u32,
) -> MutPtr<malloc_zone_t> {
    let zone_struct = malloc_zone_t::new(env);
    env.mem.create_zone(zone_struct)
}

fn malloc_destroy_zone(env: &mut Environment, zone: MutPtr<malloc_zone_t>) {
    env.mem.destroy_zone(zone);
    env.mem.free(zone.cast());
}

fn malloc_set_zone_name(
    env: &mut Environment,
    zone_ptr: MutPtr<malloc_zone_t>,
    name: ConstPtr<u8>,
) {
    let mut zone = env.mem.read(zone_ptr);

    let name = env.mem.cstr_at(name).to_owned();

    zone.zone_name = env.mem.zone_alloc_and_write_cstr(zone_ptr, &name).cast_const();

    env.mem.write(zone_ptr, zone);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(malloc_zone_malloc(_, _)),
    export_c_func!(malloc_set_zone_name(_, _)),
    export_c_func!(malloc_destroy_zone(_)),
    export_c_func!(malloc_create_zone(_, _)),
    export_c_func!(malloc_zone_free(_, _)),
    export_c_func!(malloc_zone_realloc(_, _, _)),
    export_c_func!(malloc_zone_size(_, _)),
    export_c_func!(malloc_default_zone()),
];
