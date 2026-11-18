use std::ffi::CString;
use std::os::raw::c_char;

fn main() {
    let args: Vec<CString> = std::env::args()
        .map(|arg| CString::new(arg).unwrap())
        .collect();
    let mut c_args: Vec<*mut c_char> = args
        .iter()
        .map(|arg| arg.as_ptr() as *mut c_char)
        .collect();
    c_args.push(std::ptr::null_mut());

    unsafe {
        ferrum_doom::i_main::doom_main(args.len() as i32, c_args.as_mut_ptr());
    }
}
