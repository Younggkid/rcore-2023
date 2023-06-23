window.SIDEBAR_ITEMS = {"fn":[["translated_byte_buffer","Create mutable Vec slice in kernel space from ptr in other address space. NOTICE: the content pointed to by the pointer `ptr` can cross physical pages."],["translated_ref","translate a pointer `ptr` in other address space to a immutable u8 slice in kernel address space. NOTICE: the content pointed to by the pointer `ptr` cannot cross physical pages, otherwise translated_byte_buffer should be used."],["translated_refmut","translate a pointer `ptr` in other address space to a mutable u8 slice in kernel address space. NOTICE: the content pointed to by the pointer `ptr` cannot cross physical pages, otherwise translated_byte_buffer should be used."],["translated_str","Create String in kernel address space from u8 Array(end with 0) in other address space"]],"struct":[["PTEFlags","page table entry flags"],["PageTable","page table structure"],["PageTableEntry","page table entry structure"],["UserBuffer","An abstraction over a buffer passed from user space to kernel space"],["UserBufferIterator","An iterator over a UserBuffer"]]};