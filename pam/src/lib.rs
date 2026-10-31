use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};

// PAM constants
const PAM_SUCCESS: c_int = 0;
const PAM_AUTH_ERR: c_int = 7;
const PAM_USER: c_int = 2;

// ponytail: stub PAM module — always denies auth until daemon proximity check is wired up.
// Upgrade path: connect to handoffd via Unix socket, check proximity + Face ID token.

// Raw PAM handle — opaque struct, we only use it through the vtable
#[repr(C)]
pub struct PamHandle {
    _data: [u8; 0],
}

extern "C" {
    fn pam_get_item(handle: *const PamHandle, item_type: c_int, item: *mut *const c_void) -> c_int;
}

#[no_mangle]
pub extern "C" fn pam_sm_authenticate(
    pamh: *const PamHandle,
    _flags: c_int,
    _argc: c_int,
    _argv: *const *const c_char,
) -> c_int {
    // TODO: query handoffd for nearby trusted device
    let _user = unsafe {
        let mut item: *const c_void = std::ptr::null();
        if pam_get_item(pamh, PAM_USER, &mut item) == PAM_SUCCESS && !item.is_null() {
            CStr::from_ptr(item as *const c_char).to_str().ok()
        } else {
            None
        }
    };

    PAM_AUTH_ERR
}

#[no_mangle]
pub extern "C" fn pam_sm_setcred(
    _pamh: *const PamHandle,
    _flags: c_int,
    _argc: c_int,
    _argv: *const *const c_char,
) -> c_int {
    PAM_SUCCESS
}

#[no_mangle]
pub extern "C" fn pam_sm_acct_mgmt(
    _pamh: *const PamHandle,
    _flags: c_int,
    _argc: c_int,
    _argv: *const *const c_char,
) -> c_int {
    PAM_SUCCESS
}

#[no_mangle]
pub extern "C" fn pam_sm_open_session(
    _pamh: *const PamHandle,
    _flags: c_int,
    _argc: c_int,
    _argv: *const *const c_char,
) -> c_int {
    PAM_SUCCESS
}

#[no_mangle]
pub extern "C" fn pam_sm_close_session(
    _pamh: *const PamHandle,
    _flags: c_int,
    _argc: c_int,
    _argv: *const *const c_char,
) -> c_int {
    PAM_SUCCESS
}

#[no_mangle]
pub extern "C" fn pam_sm_chauthtok(
    _pamh: *const PamHandle,
    _flags: c_int,
    _argc: c_int,
    _argv: *const *const c_char,
) -> c_int {
    PAM_SUCCESS
}
