#[cfg(test)]
mod ptr {
    use core::ptr;

    #[test]
    fn test_no_provenance() {
        let p = ptr::NonNull::<u8>::dangling();
        // memory access failed: 0x1[noalloc] is a dangling pointer (it has no provenance)
        let _ = unsafe { *p.as_ptr() }; // UB
    }

    #[test]
    fn test_oob() {
        let arr: &[u8; 4] = &[0, 1, 2, 3];
        let p = arr as *const u8;
        // 1 byte past the end is okay
        let q = unsafe { p.add(arr.len()) };
        // any further is UB: out-of-bounds pointer arithmetic
        let _ = unsafe { q.offset(1) }; // UB
    }

    #[test]
    fn test_double_drop() {
        let x = Box::new(1);
        let _y = unsafe { ptr::read(&x) };
        panic!("bad place to panic"); // UB
    }
}

#[cfg(test)]
mod validity {
    use core::mem::transmute;

    #[test]
    fn test_bad_bool() {
        let x = 2_u8;
        #[allow(clippy::transmute_int_to_bool)]
        let _: bool = unsafe { transmute(x) };
    }
}

#[cfg(test)]
mod borrows {
    use core::cell::UnsafeCell;

    extern "Rust" {
        fn miri_get_alloc_id(ptr: *const u8) -> u64;
        fn miri_print_borrow_state(alloc_id: u64, show_unnamed: bool);
    }

    fn get_alloc_id(ptr: *const u8) -> u64 {
        unsafe { miri_get_alloc_id(ptr) }
    }

    fn print_borrow_stacks(alloc_id: u64) {
        unsafe {
            miri_print_borrow_state(alloc_id, true)
        }
    }

    #[test]
    fn test_reborrow_dbg() {
        let mut val = 1_u8;
        let alloc_id = get_alloc_id(&val as *const u8);
        print_borrow_stacks(alloc_id);

        let x: *mut u8 = &mut val;
        print_borrow_stacks(alloc_id);

        // let _y: *mut u8 = unsafe { &mut *x }; // ok
        let _y: *mut u8 = &mut val; // not ok
        print_borrow_stacks(alloc_id);

        let _ = unsafe { *x };
        print_borrow_stacks(alloc_id);
    }

    /// This is UB. The parent of `_y` is `val`, which pops x off the stack.
    #[test]
    fn test_reborrow() {
        let mut val = 1_u8;
        let x: *mut u8 = &mut val;
        let _y: *mut u8 = &mut val; // not ok
        let _ = unsafe { *x }; // UB
    }

    /// This is not UB. The parent of `_y` is now `x`, so `x` stays on the stack.
    #[test]
    fn test_ok_reborrow() {
        let mut val = 1_u8;
        let x: *mut u8 = &mut val;
        let _y: *mut u8 = unsafe { &mut *x }; // ok
        let _ = unsafe { *x };
    }

    #[test]
    fn test_cell() {
        let x = 0_usize;
        let cell = &x as *const usize as *const UnsafeCell<usize>;
        let _ = unsafe { &*cell }; // UB
    }

    #[test]
    fn test_ok_interleave_reads() {
        let mut val = 1_u8;
        let u: *mut u8 = &mut val;
        let s: *const u8 = unsafe { &*u };
        let _ = unsafe { *s };
        let _ = unsafe { *u };
        let _ = unsafe { *s };
    }

}
