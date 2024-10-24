mod protocol;
mod tbon;
mod utils;

use std::sync::Arc;

use crate::tbon::Tbon;

#[no_mangle]
pub unsafe extern "C" fn treemon_root_init() -> *mut Tbon {
    let mut tbon = Tbon::init_as_root(None).unwrap();
    return Arc::into_raw(Arc::new(tbon)) as *mut Tbon;
}

#[no_mangle]
pub unsafe extern "C" fn treemon_leaf_init() -> *mut Tbon {
    let mut tbon = Tbon::init_as_leaf().unwrap();
    return Arc::into_raw(Arc::new(tbon)) as *mut Tbon;
}

#[no_mangle]
pub unsafe extern "C" fn treemon_set_counter(
    tbon: *mut Tbon,
    cnt: *const std::os::raw::c_char,
    value: u64,
) -> i32 {
    let tbon: &mut Tbon = unsafe { &mut *(tbon) };

    let cnt = unsafe { std::ffi::CStr::from_ptr(cnt) };
    tbon.set_counter(cnt.to_str().unwrap(), value);

    0
}
