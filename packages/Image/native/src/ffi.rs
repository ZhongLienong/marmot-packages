use crate::images;
use std::cell::RefCell;
use std::ffi::{CStr, c_void};
use std::path::Path;

thread_local! {
    static ERROR: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn complete(result: Result<i64, String>) -> i64 {
    ERROR.with(|error| match result {
        Ok(value) => {
            error.borrow_mut().clear();
            value
        }
        Err(message) => {
            *error.borrow_mut() = message.chars().map(u32::from).collect();
            -1
        }
    })
}

unsafe fn integer(args: *const *const c_void, index: usize) -> i64 {
    unsafe { *args.add(index) as isize as i64 }
}

unsafe fn text(args: *const *const c_void, index: usize) -> Result<String, String> {
    let text = unsafe { CStr::from_ptr((*args.add(index)).cast()) };
    text.to_str()
        .map(str::to_owned)
        .map_err(|error| format!("path is not UTF-8: {error}"))
}

macro_rules! entry {
    ($name:ident($args:ident) $body:block) => {
        /// # Safety
        /// `args` must contain the declared Marmot ABI v1 argument slots, and
        /// `ret` must point to writable, aligned storage for an eight-byte result.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($args: *const *const c_void, ret: *mut c_void) {
            let value: i64 = $body;
            unsafe {
                ret.cast::<i64>().write(value);
            }
        }
    };
}

entry! { marmot_image_create(args) {
    complete(images::create(unsafe { integer(args, 0) }, unsafe { integer(args, 1) }, unsafe { integer(args, 2) }))
} }

entry! { marmot_image_read(args) {
    complete(unsafe { text(args, 0) }.and_then(|path| images::read(Path::new(&path))))
} }

entry! { marmot_image_width(args) {
    complete(images::width(unsafe { integer(args, 0) }))
} }

entry! { marmot_image_height(args) {
    complete(images::height(unsafe { integer(args, 0) }))
} }

entry! { marmot_image_get_pixel(args) {
    complete(images::get_pixel(unsafe { integer(args, 0) }, unsafe { integer(args, 1) }, unsafe { integer(args, 2) }))
} }

entry! { marmot_image_set_pixel(args) {
    complete(images::set_pixel(unsafe { integer(args, 0) }, unsafe { integer(args, 1) }, unsafe { integer(args, 2) }, unsafe { integer(args, 3) }))
} }

entry! { marmot_image_write(args) {
    complete(unsafe { text(args, 1) }.and_then(|path| images::write(unsafe { integer(args, 0) }, Path::new(&path), None)))
} }

entry! { marmot_image_write_jpeg(args) {
    complete(unsafe { text(args, 1) }.and_then(|path| images::write(unsafe { integer(args, 0) }, Path::new(&path), Some(unsafe { integer(args, 2) }))))
} }

entry! { marmot_image_resize(args) {
    complete(images::resize(unsafe { integer(args, 0) }, unsafe { integer(args, 1) }, unsafe { integer(args, 2) }))
} }

entry! { marmot_image_crop(args) {
    complete(images::crop(unsafe { integer(args, 0) }, unsafe { integer(args, 1) }, unsafe { integer(args, 2) }, unsafe { integer(args, 3) }, unsafe { integer(args, 4) }))
} }

entry! { marmot_image_flip_horizontal(args) {
    complete(images::flip_horizontal(unsafe { integer(args, 0) }))
} }

entry! { marmot_image_flip_vertical(args) {
    complete(images::flip_vertical(unsafe { integer(args, 0) }))
} }

entry! { marmot_image_close(args) {
    complete(images::close(unsafe { integer(args, 0) }))
} }

// Errors cross the boundary as Unicode scalars, keeping allocations in their
// owning runtime even when the VM and DLL use different C runtimes.
entry! { marmot_image_error_length(_args) {
    ERROR.with(|error| error.borrow().len() as i64)
} }

entry! { marmot_image_error_character(args) {
    let index = unsafe { integer(args, 0) } as usize;
    ERROR.with(|error| i64::from(error.borrow()[index]))
} }

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn ffi_uses_scalar_slots_and_keeps_unicode_errors_local_to_each_thread() {
        let workers: Vec<_> = (0..4)
            .map(|index| {
                std::thread::spawn(move || {
                    let path = CString::new(format!("missing-image-é-{index}.png")).unwrap();
                    let args = [path.as_ptr().cast::<c_void>()];
                    let mut result = 0i64;
                    unsafe {
                        marmot_image_read(args.as_ptr(), (&mut result as *mut i64).cast());
                    }
                    assert_eq!(result, -1);
                    unsafe {
                        marmot_image_error_length(
                            std::ptr::null(),
                            (&mut result as *mut i64).cast(),
                        );
                    }
                    let length = result;
                    let mut message = String::new();
                    for offset in 0..length {
                        let args = [offset as usize as *const c_void];
                        unsafe {
                            marmot_image_error_character(
                                args.as_ptr(),
                                (&mut result as *mut i64).cast(),
                            );
                        }
                        message.push(char::from_u32(result as u32).unwrap());
                    }
                    assert!(message.contains(&format!("missing-image-é-{index}.png")));
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }

        let args = [
            2usize as *const c_void,
            3usize as *const c_void,
            0x12345678usize as *const c_void,
        ];
        let mut result = 0i64;
        unsafe {
            marmot_image_create(args.as_ptr(), (&mut result as *mut i64).cast());
        }
        assert!(result > 0);
        assert_eq!(images::get_pixel(result, 1, 2).unwrap(), 0x12345678);
        images::close(result).unwrap();
    }
}
