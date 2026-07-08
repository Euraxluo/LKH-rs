#[cfg(feature = "unsafe-ffi")]
fn main() {
    // Raw FFI is exposed only as an escape hatch. Prefer lkh_rs::solve_parameter_file.
    unsafe {
        println!("LKH clock: {}", lkh_rs::ffi::GetTime());
    }
}

#[cfg(not(feature = "unsafe-ffi"))]
fn main() {
    eprintln!("Run with `--features unsafe-ffi` to access raw bindgen symbols.");
}
