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
        let _y: Box<usize> = unsafe { ptr::read(&x) };
        panic!("bad place to panic") // UB
    }
}

mod borrows {
    use core::cell::UnsafeCell;

    #[test]
    fn test_cell() {
        let x = &0_usize as *const usize;
        let cell = x as *const UnsafeCell<usize>;
        let _ = unsafe { &*cell }; // UB
    }
}
