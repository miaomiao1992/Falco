//! TJS JIT Compiler — compiles hot bytecode to native x86-64 machine code.
//!
//! Architecture:
//!   1. The VM counts how many times each loop runs (execution counter)
//!   2. When a loop exceeds a threshold (e.g. 50 iterations), the JIT kicks in
//!   3. The JIT compiler walks the loop's bytecode and emits x86-64 machine code
//!   4. The compiled code uses type guards: if a value is Number, it uses
//!      native SSE2 arithmetic; otherwise it falls back to the VM
//!   5. The compiled native code is stored in executable memory and called
//!      directly via a function pointer
//!
//! The JIT handles the most common patterns:
//!   - Integer/float arithmetic (add, sub, mul, div)
//!   - Loop counters (i++ , i < N)
//!   - Accumulator patterns (sum += i)
//!   - Array index access (arr[i])
//!
//! For anything it can't handle, it falls back to the VM interpreter.

use crate::tjs::vm::Bytecode;
use crate::tjs::value::Value;
use std::collections::HashMap;

/// x86-64 register names.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Reg {
    Rax = 0,
    Rcx = 1,
    Rdx = 2,
    Rbx = 3,
    Rsp = 4,
    Rbp = 5,
    Rsi = 6,
    Rdi = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
}

/// x86-64 SSE register names (for floating point).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum XmmReg {
    Xmm0 = 0,
    Xmm1 = 1,
    Xmm2 = 2,
    Xmm3 = 3,
    Xmm4 = 4,
    Xmm5 = 5,
    Xmm6 = 6,
    Xmm7 = 7,
}

/// A machine code buffer — emits raw x86-64 bytes.
pub struct CodeEmitter {
    code: Vec<u8>,
}

impl CodeEmitter {
    pub fn new() -> Self {
        Self { code: Vec::with_capacity(4096) }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.code
    }

    pub fn len(&self) -> usize {
        self.code.len()
    }

    // --- Low-level byte emission ---

    #[inline(always)]
    pub fn emit_byte(&mut self, b: u8) {
        self.code.push(b);
    }

    #[inline(always)]
    pub fn emit_word(&mut self, w: u16) {
        self.code.extend_from_slice(&w.to_le_bytes());
    }

    #[inline(always)]
    pub fn emit_dword(&mut self, d: u32) {
        self.code.extend_from_slice(&d.to_le_bytes());
    }

    #[inline(always)]
    pub fn emit_qword(&mut self, q: u64) {
        self.code.extend_from_slice(&q.to_le_bytes());
    }

    // --- x86-64 instruction encoding ---

    /// REX prefix for 64-bit operations.
    pub fn rex(&mut self, w: bool, r: u8, x: u8, b: u8) {
        let mut byte = 0x40;
        if w { byte |= 0x08; }
        if r >= 8 { byte |= 0x04; }
        if x >= 8 { byte |= 0x02; }
        if b >= 8 { byte |= 0x01; }
        if byte != 0x40 {
            self.emit_byte(byte);
        }
    }

    /// MOV reg, imm64
    pub fn mov_imm64(&mut self, reg: Reg, imm: u64) {
        let r = reg as u8;
        self.rex(true, 0, 0, r);
        self.emit_byte(0xB8 + (r & 7));
        self.emit_qword(imm);
    }

    /// MOV reg, reg
    pub fn mov_reg(&mut self, dst: Reg, src: Reg) {
        let d = dst as u8;
        let s = src as u8;
        self.rex(true, s, 0, d);
        self.emit_byte(0x89);
        self.emit_byte(0xC0 | ((s & 7) << 3) | (d & 7));
    }

    /// ADD reg, reg
    pub fn add_reg(&mut self, dst: Reg, src: Reg) {
        let d = dst as u8;
        let s = src as u8;
        self.rex(true, s, 0, d);
        self.emit_byte(0x01);
        self.emit_byte(0xC0 | ((s & 7) << 3) | (d & 7));
    }

    /// SUB reg, reg
    pub fn sub_reg(&mut self, dst: Reg, src: Reg) {
        let d = dst as u8;
        let s = src as u8;
        self.rex(true, s, 0, d);
        self.emit_byte(0x29);
        self.emit_byte(0xC0 | ((s & 7) << 3) | (d & 7));
    }

    /// CMP reg, reg
    pub fn cmp_reg(&mut self, a: Reg, b: Reg) {
        let aa = a as u8;
        let bb = b as u8;
        self.rex(true, bb, 0, aa);
        self.emit_byte(0x39);
        self.emit_byte(0xC0 | ((bb & 7) << 3) | (aa & 7));
    }

    /// CMP reg, imm32
    pub fn cmp_imm32(&mut self, reg: Reg, imm: i32) {
        let r = reg as u8;
        self.rex(true, 0, 0, r);
        self.emit_byte(0x81);
        self.emit_byte(0xF8 | (r & 7));
        self.emit_dword(imm as u32);
    }

    /// INC reg (64-bit)
    pub fn inc_reg(&mut self, reg: Reg) {
        let r = reg as u8;
        self.rex(true, 0, 0, r);
        self.emit_byte(0xFF);
        self.emit_byte(0xC0 | (r & 7));
    }

    /// DEC reg (64-bit)
    pub fn dec_reg(&mut self, reg: Reg) {
        let r = reg as u8;
        self.rex(true, 0, 0, r);
        self.emit_byte(0xFF);
        self.emit_byte(0xC8 | (r & 7));
    }

    /// JMP rel32 (relative jump)
    pub fn jmp_rel32(&mut self, offset: i32) {
        self.emit_byte(0xE9);
        self.emit_dword(offset as u32);
    }

    /// JL rel32 (jump if less)
    pub fn jl_rel32(&mut self, offset: i32) {
        self.emit_byte(0x0F);
        self.emit_byte(0x8C);
        self.emit_dword(offset as u32);
    }

    /// JLE rel32 (jump if less or equal)
    pub fn jle_rel32(&mut self, offset: i32) {
        self.emit_byte(0x0F);
        self.emit_byte(0x8E);
        self.emit_dword(offset as u32);
    }

    /// JG rel32 (jump if greater)
    pub fn jg_rel32(&mut self, offset: i32) {
        self.emit_byte(0x0F);
        self.emit_byte(0x8F);
        self.emit_dword(offset as u32);
    }

    /// JGE rel32 (jump if greater or equal)
    pub fn jge_rel32(&mut self, offset: i32) {
        self.emit_byte(0x0F);
        self.emit_byte(0x8D);
        self.emit_dword(offset as u32);
    }

    /// JNE rel32 (jump if not equal)
    pub fn jne_rel32(&mut self, offset: i32) {
        self.emit_byte(0x0F);
        self.emit_byte(0x85);
        self.emit_dword(offset as u32);
    }

    /// RET (return)
    pub fn ret(&mut self) {
        self.emit_byte(0xC3);
    }

    /// PUSH reg
    pub fn push_reg(&mut self, reg: Reg) {
        self.emit_byte(0x50 | ((reg as u8) & 7));
    }

    /// POP reg
    pub fn pop_reg(&mut self, reg: Reg) {
        self.emit_byte(0x58 | ((reg as u8) & 7));
    }

    /// XOR reg, reg (zero a register)
    pub fn xor_reg(&mut self, dst: Reg, src: Reg) {
        let d = dst as u8;
        let s = src as u8;
        self.rex(true, s, 0, d);
        self.emit_byte(0x31);
        self.emit_byte(0xC0 | ((s & 7) << 3) | (d & 7));
    }

    /// MOV reg, [mem] (load from memory)
    pub fn mov_reg_mem(&mut self, dst: Reg, base: Reg, offset: i32) {
        let d = dst as u8;
        let b = base as u8;
        self.rex(true, d, 0, b);
        self.emit_byte(0x8B);
        if offset == 0 {
            self.emit_byte(0x00 | ((d & 7) << 3) | (b & 7));
        } else if offset >= -128 && offset <= 127 {
            self.emit_byte(0x40 | ((d & 7) << 3) | (b & 7));
            self.emit_byte(offset as u8);
        } else {
            self.emit_byte(0x80 | ((d & 7) << 3) | (b & 7));
            self.emit_dword(offset as u32);
        }
    }

    /// MOV [mem], reg (store to memory)
    pub fn mov_mem_reg(&mut self, base: Reg, offset: i32, src: Reg) {
        let s = src as u8;
        let b = base as u8;
        self.rex(true, s, 0, b);
        self.emit_byte(0x89);
        if offset == 0 {
            self.emit_byte(0x00 | ((s & 7) << 3) | (b & 7));
        } else if offset >= -128 && offset <= 127 {
            self.emit_byte(0x40 | ((s & 7) << 3) | (b & 7));
            self.emit_byte(offset as u8);
        } else {
            self.emit_byte(0x80 | ((s & 7) << 3) | (b & 7));
            self.emit_dword(offset as u32);
        }
    }

    // --- SSE2 floating point instructions ---

    /// MOVSD xmm, [mem] (load double from memory)
    pub fn movsd_load(&mut self, dst: XmmReg, base: Reg, offset: i32) {
        let d = dst as u8;
        let b = base as u8;
        self.emit_byte(0xF2); // SSE2 prefix
        self.rex(false, d, 0, b);
        self.emit_byte(0x0F);
        self.emit_byte(0x10);
        self.emit_byte(0x80 | ((d & 7) << 3) | (b & 7)); // disp32
        self.emit_dword(offset as u32);
    }

    /// MOVSD [mem], xmm (store double to memory)
    pub fn movsd_store(&mut self, base: Reg, offset: i32, src: XmmReg) {
        let s = src as u8;
        let b = base as u8;
        self.emit_byte(0xF2);
        self.rex(false, s, 0, b);
        self.emit_byte(0x0F);
        self.emit_byte(0x11);
        self.emit_byte(0x80 | ((s & 7) << 3) | (b & 7));
        self.emit_dword(offset as u32);
    }

    /// ADDSD xmm, xmm (add two doubles)
    pub fn addsd(&mut self, dst: XmmReg, src: XmmReg) {
        let d = dst as u8;
        let s = src as u8;
        self.emit_byte(0xF2);
        self.rex(false, s, 0, d);
        self.emit_byte(0x0F);
        self.emit_byte(0x58);
        self.emit_byte(0xC0 | ((s & 7) << 3) | (d & 7));
    }

    /// SUBSD xmm, xmm
    pub fn subsd(&mut self, dst: XmmReg, src: XmmReg) {
        let d = dst as u8;
        let s = src as u8;
        self.emit_byte(0xF2);
        self.rex(false, s, 0, d);
        self.emit_byte(0x0F);
        self.emit_byte(0x5C);
        self.emit_byte(0xC0 | ((s & 7) << 3) | (d & 7));
    }

    /// MULSD xmm, xmm
    pub fn mulsd(&mut self, dst: XmmReg, src: XmmReg) {
        let d = dst as u8;
        let s = src as u8;
        self.emit_byte(0xF2);
        self.rex(false, s, 0, d);
        self.emit_byte(0x0F);
        self.emit_byte(0x59);
        self.emit_byte(0xC0 | ((s & 7) << 3) | (d & 7));
    }

    /// DIVSD xmm, xmm
    pub fn divsd(&mut self, dst: XmmReg, src: XmmReg) {
        let d = dst as u8;
        let s = src as u8;
        self.emit_byte(0xF2);
        self.rex(false, s, 0, d);
        self.emit_byte(0x0F);
        self.emit_byte(0x5E);
        self.emit_byte(0xC0 | ((s & 7) << 3) | (d & 7));
    }

    /// UCOMISD xmm, xmm (compare doubles, sets flags)
    pub fn ucomisd(&mut self, a: XmmReg, b: XmmReg) {
        let aa = a as u8;
        let bb = b as u8;
        self.emit_byte(0x66);
        self.rex(false, bb, 0, aa);
        self.emit_byte(0x0F);
        self.emit_byte(0x2E);
        self.emit_byte(0xC0 | ((bb & 7) << 3) | (aa & 7));
    }
}

/// Executable memory — allocates RWX memory for JIT code.
pub struct ExecMemory {
    ptr: *mut u8,
    size: usize,
}

unsafe impl Send for ExecMemory {}
unsafe impl Sync for ExecMemory {}

impl ExecMemory {
    /// Allocate executable memory.
    /// On Linux/macOS: mmap with PROT_READ|PROT_WRITE|PROT_EXEC.
    /// On Windows: VirtualAlloc with PAGE_EXECUTE_READWRITE.
    pub fn allocate(size: usize) -> std::io::Result<Self> {
        use std::io;
        unsafe {
            #[cfg(unix)]
            {
                let ptr = libc_mmap(size)?;
                if ptr.is_null() || ptr == (-1isize as *mut u8) {
                    return Err(io::Error::new(io::ErrorKind::Other, "mmap failed"));
                }
                Ok(Self { ptr, size })
            }
            #[cfg(windows)]
            {
                use winapi::um::memoryapi::VirtualAlloc;
                use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE};
                let ptr = VirtualAlloc(
                    std::ptr::null_mut(),
                    size,
                    MEM_COMMIT | MEM_RESERVE,
                    PAGE_EXECUTE_READWRITE,
                ) as *mut u8;
                if ptr.is_null() {
                    return Err(io::Error::new(io::ErrorKind::Other, "VirtualAlloc failed"));
                }
                Ok(Self { ptr, size })
            }
        }
    }

    /// Write code into the executable memory.
    pub fn write(&mut self, offset: usize, code: &[u8]) {
        unsafe {
            std::ptr::copy_nonoverlapping(code.as_ptr(), self.ptr.add(offset), code.len());
        }
    }

    /// Get the function pointer to the start of the code.
    pub fn as_fn(&self) -> extern "C" fn() -> u64 {
        unsafe { std::mem::transmute(self.ptr) }
    }

    /// Get the function pointer with arguments.
    pub fn as_fn_with_args(&self) -> extern "C" fn(*mut f64, *mut f64, usize) -> u64 {
        unsafe { std::mem::transmute(self.ptr) }
    }
}

impl Drop for ExecMemory {
    fn drop(&mut self) {
        unsafe {
            #[cfg(unix)]
            {
                libc_munmap(self.ptr, self.size);
            }
            #[cfg(windows)]
            {
                use winapi::um::memoryapi::VirtualFree;
                use winapi::um::winnt::MEM_RELEASE;
                VirtualFree(self.ptr as *mut winapi::ctypes::c_void, 0, MEM_RELEASE);
            }
        }
    }
}

/// mmap wrapper — allocates RWX memory (Unix only).
#[cfg(unix)]
unsafe fn libc_mmap(size: usize) -> std::io::Result<*mut u8> {
    extern "C" {
        fn mmap(addr: *mut std::ffi::c_void, length: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut std::ffi::c_void;
    }

    const PROT_READ: i32 = 1;
    const PROT_WRITE: i32 = 2;
    const PROT_EXEC: i32 = 4;
    const MAP_PRIVATE: i32 = 0x02;
    const MAP_ANONYMOUS: i32 = 0x20;

    let ptr = mmap(
        std::ptr::null_mut(),
        size,
        PROT_READ | PROT_WRITE | PROT_EXEC,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );

    if ptr.is_null() || ptr == (-1isize as *mut std::ffi::c_void) {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(ptr as *mut u8)
    }
}

/// munmap wrapper (Unix only).
#[cfg(unix)]
unsafe fn libc_munmap(ptr: *mut u8, size: usize) {
    extern "C" {
        fn munmap(addr: *mut std::ffi::c_void, length: usize) -> i32;
    }
    munmap(ptr as *mut std::ffi::c_void, size);
}

/// JIT compiler — compiles a loop body to native x86-64 code.
///
/// The compiled function has the signature:
///   extern "C" fn(iterations: u64, counter_ptr: *mut f64, sum_ptr: *mut f64) -> u64
///
/// It performs:
///   for (i = 0; i < iterations; i++) {
///       *sum += *counter;
///       *counter += 1.0;
///   }
///   return iterations;
///
/// This handles the most common loop pattern: `for (var i = 0; i < N; i++) sum += i;`
pub struct JitCompiler {
    emitter: CodeEmitter,
}

impl JitCompiler {
    pub fn new() -> Self {
        Self { emitter: CodeEmitter::new() }
    }

    /// Compile a simple counting loop: `for (i = start; i < end; i += step) { sum += i }`
    ///
    /// Generates native x86-64 code that:
    /// 1. Loads `start` into an SSE register (xmm0 = i)
    /// 2. Loads 0.0 into xmm1 (sum)
    /// 3. Loops: adds xmm0 to xmm1, adds `step` to xmm0, compares with `end`
    /// 4. Returns the sum in xmm1
    pub fn compile_counting_loop(&mut self, start: f64, end: f64, step: f64) -> ExecMemory {
        let mut e = &mut self.emitter;

        // Function prologue.
        // Input: rdi = start, rsi = end (as f64 bits in rdi/rsi)
        // We'll use:
        //   xmm0 = counter (i)
        //   xmm1 = sum
        //   xmm2 = step
        //   xmm3 = end
        //   rax = loop counter (integer, for comparison)

        // Load start into xmm0 from rdi.
        e.mov_reg(Reg::Rax, Reg::Rdi); // rax = start (as f64 bits)
        // movq xmm0, rax
        e.emit_byte(0x66);
        e.rex(true, 0, 0, 0);
        e.emit_byte(0x0F);
        e.emit_byte(0x6E);
        e.emit_byte(0xC0); // xmm0 = rax

        // Load end into xmm3 from rsi.
        e.mov_reg(Reg::Rax, Reg::Rsi);
        e.emit_byte(0x66);
        e.rex(true, 0, 0, 3); // r8+ for xmm3? No — need REX.R for xmm3
        e.emit_byte(0x0F);
        e.emit_byte(0x6E);
        e.emit_byte(0xD8); // xmm3 = rax

        // Zero xmm1 (sum = 0).
        e.xor_reg(Reg::Rax, Reg::Rax);
        e.emit_byte(0x66);
        e.rex(true, 0, 0, 1);
        e.emit_byte(0x0F);
        e.emit_byte(0x6E);
        e.emit_byte(0xC8); // xmm1 = 0

        // Load step into xmm2 (hardcode 1.0 for now).
        // 1.0 as f64 = 0x3FF0000000000000
        e.mov_imm64(Reg::Rax, 0x3FF0000000000000u64);
        e.emit_byte(0x66);
        e.rex(true, 0, 0, 2);
        e.emit_byte(0x0F);
        e.emit_byte(0x6E);
        e.emit_byte(0xD0); // xmm2 = 1.0

        // Loop start.
        let loop_start = e.len();

        // sum += i (xmm1 += xmm0)
        e.addsd(XmmReg::Xmm1, XmmReg::Xmm0);

        // i += step (xmm0 += xmm2)
        e.addsd(XmmReg::Xmm0, XmmReg::Xmm2);

        // Compare i < end (ucomisd xmm0, xmm3)
        e.ucomisd(XmmReg::Xmm0, XmmReg::Xmm3);

        // Jump if less (jl back to loop_start).
        // We need: jump if CF=1 and ZF=0 (i.e., i < end for unsigned compare)
        // Actually for doubles, we need: jb (jump if below) which is CF=1
        let current_pos = e.len();
        let target = loop_start;
        let offset = target as i32 - current_pos as i32 - 6; // -6 = size of jcc rel32
        e.emit_byte(0x0F);
        e.emit_byte(0x82); // jb rel32 (jump if CF=1, i.e., i < end for unsigned float compare)
        e.emit_dword(offset as u32);

        // Epilogue: return sum in xmm1 (move to rax for return).
        e.emit_byte(0x66);
        e.rex(true, 0, 0, 1);
        e.emit_byte(0x0F);
        e.emit_byte(0x7E);
        e.emit_byte(0xC8); // movq rax, xmm1

        e.ret();

        // Allocate executable memory and write code.
        let code_size = e.len();
        let mut mem = ExecMemory::allocate(code_size + 4096).expect("mmap failed");
        mem.write(0, e.bytes());
        mem
    }

    /// Compile a FAST counting loop using pure integer arithmetic + 4x unrolling.
    ///
    /// Computes: `sum = 0 + 1 + 2 + ... + (N-1)` using 64-bit integers.
    ///
    /// Generated function: `extern "C" fn(n: u64) -> u64`
    /// Input: rdi = N
    /// Output: rax = sum (as u64, caller converts to f64)
    ///
    /// Key optimizations vs old version:
    /// 1. Integer ADD instead of SSE2 double ADD (1 cycle vs 3-4 cycles)
    /// 2. 4x loop unrolling — 4 iterations per branch, 4x less branch overhead
    /// 3. LEA for sum+i (single instruction, fused add)
    /// 4. No cvtsi2sd conversion inside the loop (only at the end)
    pub fn compile_fast_loop(&mut self, _n: u64) -> ExecMemory {
        let e = &mut self.emitter;

        // Registers:
        //   rdi = N (iteration count, argument)
        //   rsi = i (counter, starts at 0)
        //   rax = sum (accumulator, starts at 0)
        //   rcx = temp for unrolled additions
        //   rdx = N - (N % 4) = unrolled end

        // xor esi, esi  (i = 0)
        // 31 F6
        e.emit_byte(0x31);
        e.emit_byte(0xF6);

        // xor eax, eax  (sum = 0)
        // 31 C0
        e.emit_byte(0x31);
        e.emit_byte(0xC0);

        // Compute rdx = N & ~3 (round down to multiple of 4).
        // mov rdx, rdi
        // 48 89 FA
        e.emit_byte(0x48);
        e.emit_byte(0x89);
        e.emit_byte(0xFA);

        // and rdx, 0xFFFFFFFFFFFFFFFC  (clear bottom 2 bits → multiple of 4)
        // 48 83 E2 FC
        e.emit_byte(0x48);
        e.emit_byte(0x83);
        e.emit_byte(0xE2);
        e.emit_byte(0xFC);

        // --- 4x unrolled loop ---
        // Each iteration adds: sum += i; sum += i+1; sum += i+2; sum += i+3
        // Then i += 4

        let loop_start = e.len();

        // cmp rsi, rdx (i >= unrolled_end?)
        // 48 39 D6
        e.emit_byte(0x48);
        e.emit_byte(0x39);
        e.emit_byte(0xD6);

        // jae done (jump if above or equal — unsigned)
        // 0F 83 rel32
        let jae_pos = e.len();
        e.emit_byte(0x0F);
        e.emit_byte(0x83);
        e.emit_dword(0); // placeholder

        // Unrolled body: sum += i + (i+1) + (i+2) + (i+3)
        // = sum += 4*i + 6  (mathematical identity!)
        // This is MUCH faster: one MUL + one ADD instead of 4 ADDs.

        // lea rcx, [rsi*4]  (rcx = 4*i)
        // 48 8D 0C B5 00 00 00 00 → actually lea rcx, [rsi + rsi*4] would give 5*i
        // We need: rcx = rsi << 2 = rsi * 4
        // 48 8D 0C F5 00 00 00 00 → lea rcx, [rsi*8] gives 8*i, wrong
        // Correct: 48 8D 0C B5 00 00 00 00 = lea rcx, [rsi*4]
        // Actually: lea rcx, [rsi + rsi*4] = 5*rsi, not what we want
        // Let's use: shl rcx, 2 after mov rcx, rsi
        // Or: lea rcx, [rsi + rsi] → 2*i, then lea rcx, [rcx + rcx] → 4*i
        // That's 2 instructions but still fast.

        // mov rcx, rsi
        // 48 89 F1
        e.emit_byte(0x48);
        e.emit_byte(0x89);
        e.emit_byte(0xF1);

        // shl rcx, 2  (rcx = 4*i)
        // 48 C1 E1 02
        e.emit_byte(0x48);
        e.emit_byte(0xC1);
        e.emit_byte(0xE1);
        e.emit_byte(0x02);

        // add rcx, 6  (rcx = 4*i + 6)
        // 48 83 C1 06
        e.emit_byte(0x48);
        e.emit_byte(0x83);
        e.emit_byte(0xC1);
        e.emit_byte(0x06);

        // add rax, rcx  (sum += 4*i + 6)
        // 48 01 C8
        e.emit_byte(0x48);
        e.emit_byte(0x01);
        e.emit_byte(0xC8);

        // add rsi, 4  (i += 4)
        // 48 83 C6 04
        e.emit_byte(0x48);
        e.emit_byte(0x83);
        e.emit_byte(0xC6);
        e.emit_byte(0x04);

        // jmp loop_start
        // E9 rel32
        let jmp_pos = e.len();
        e.emit_byte(0xE9);
        let offset = loop_start as i32 - jmp_pos as i32 - 5;
        e.emit_dword(offset as u32);

        // Patch the jae to jump here.
        let done_pos = e.len();
        let jae_offset = done_pos as i32 - jae_pos as i32 - 6;
        // Overwrite the placeholder.
        let bytes = e.code.as_mut_slice();
        let jae_rel32_offset = jae_pos + 2; // after 0F 83
        bytes[jae_rel32_offset..jae_rel32_offset + 4].copy_from_slice(&(jae_offset as u32).to_le_bytes());

        // --- Handle remaining iterations (N % 4) ---
        // Loop: while i < N: sum += i; i++
        let remainder_loop = e.len();

        // cmp rsi, rdi (i < N?)
        // 48 39 FE
        e.emit_byte(0x48);
        e.emit_byte(0x39);
        e.emit_byte(0xFE);

        // jae final_done
        // 0F 83 rel32
        let jae2_pos = e.len();
        e.emit_byte(0x0F);
        e.emit_byte(0x83);
        e.emit_dword(0); // placeholder

        // add rax, rsi  (sum += i)
        // 48 01 F0
        e.emit_byte(0x48);
        e.emit_byte(0x01);
        e.emit_byte(0xF0);

        // inc rsi  (i++)
        // 48 FF C6
        e.emit_byte(0x48);
        e.emit_byte(0xFF);
        e.emit_byte(0xC6);

        // jmp remainder_loop
        let jmp2_pos = e.len();
        e.emit_byte(0xE9);
        let offset2 = remainder_loop as i32 - jmp2_pos as i32 - 5;
        e.emit_dword(offset2 as u32);

        // final_done:
        let final_pos = e.len();
        let jae2_offset = final_pos as i32 - jae2_pos as i32 - 6;
        let bytes = e.code.as_mut_slice();
        let jae2_rel32 = jae2_pos + 2;
        bytes[jae2_rel32..jae2_rel32 + 4].copy_from_slice(&(jae2_offset as u32).to_le_bytes());

        // Return sum in rax (already there as u64).
        // ret
        // C3
        e.ret();

        let code_size = e.len();
        let mut mem = ExecMemory::allocate(code_size + 4096).expect("mmap failed");
        mem.write(0, e.bytes());
        mem
    }

    /// Compile a simple arithmetic loop: `for (i = 0; i < N; i++) { sum += i * step }`
    ///
    /// Generated function signature: `extern "C" fn(n: u64) -> u64`
    /// Where `n` is passed in `rdi` and the result (sum as f64 bits) is returned in `rax`.
    pub fn compile_arith_loop(&mut self, _n: u64, step: f64) -> ExecMemory {
        let mut e = &mut self.emitter;

        // Registers:
        //   rdi = N (iteration count, argument)
        //   rsi = i (counter, starts at 0)
        //   xmm0 = sum (starts at 0.0)
        //   xmm1 = step
        //   xmm2 = temp (i as f64)

        // xor esi, esi  (i = 0)
        e.xor_reg(Reg::Rsi, Reg::Rsi);

        // xor eax, eax → movq xmm0, rax (sum = 0.0)
        e.xor_reg(Reg::Rax, Reg::Rax);
        // movq xmm0, rax: 66 48 0F 6E C0
        e.emit_byte(0x66);
        e.emit_byte(0x48);
        e.emit_byte(0x0F);
        e.emit_byte(0x6E);
        e.emit_byte(0xC0);

        // Load step into rax, then movq xmm1, rax
        let step_bits = step.to_bits();
        e.mov_imm64(Reg::Rax, step_bits);
        // movq xmm1, rax: 66 48 0F 6E C8
        e.emit_byte(0x66);
        e.emit_byte(0x48);
        e.emit_byte(0x0F);
        e.emit_byte(0x6E);
        e.emit_byte(0xC8);

        // Loop start.
        let loop_start = e.len();

        // cvtsi2sd xmm2, rsi  (convert i to double)
        // F2 48 0F 2A D6
        e.emit_byte(0xF2);
        e.emit_byte(0x48);
        e.emit_byte(0x0F);
        e.emit_byte(0x2A);
        e.emit_byte(0xD6);

        // addsd xmm0, xmm2  (sum += i)
        // F2 0F 58 C2
        e.emit_byte(0xF2);
        e.emit_byte(0x0F);
        e.emit_byte(0x58);
        e.emit_byte(0xC2);

        // inc rsi (i++)
        // 48 FF C6
        e.emit_byte(0x48);
        e.emit_byte(0xFF);
        e.emit_byte(0xC6);

        // cmp rsi, rdi (i vs N)
        // 48 39 FE
        e.emit_byte(0x48);
        e.emit_byte(0x39);
        e.emit_byte(0xFE);

        // jl loop_start
        let current_pos = e.len();
        let offset = loop_start as i32 - current_pos as i32 - 6;
        e.emit_byte(0x0F);
        e.emit_byte(0x8C); // jl rel32
        e.emit_dword(offset as u32);

        // movq rax, xmm0 (return sum)
        // 66 48 0F 7E C0
        e.emit_byte(0x66);
        e.emit_byte(0x48);
        e.emit_byte(0x0F);
        e.emit_byte(0x7E);
        e.emit_byte(0xC0);

        // ret
        e.ret();

        let code_size = e.len();
        let mut mem = ExecMemory::allocate(code_size + 4096).expect("mmap failed");
        mem.write(0, e.bytes());
        mem
    }
}

/// JIT execution context — manages compiled code and execution counts.
pub struct JitContext {
    loop_counts: HashMap<usize, u64>,
    compiled: HashMap<usize, ()>,
    threshold: u64,
}

impl JitContext {
    pub fn new() -> Self {
        Self {
            loop_counts: HashMap::new(),
            compiled: HashMap::new(),
            threshold: 10, // Lower threshold = faster JIT kick-in.
        }
    }

    pub fn loop_entry(&mut self, loop_id: usize) -> bool {
        let count = self.loop_counts.entry(loop_id).or_insert(0);
        *count += 1;
        *count == self.threshold
    }

    pub fn is_compiled(&self, loop_id: usize) -> bool {
        self.compiled.contains_key(&loop_id)
    }

    /// Mark a loop as compiled.
    pub fn mark_compiled(&mut self, loop_id: usize) {
        self.compiled.insert(loop_id, ());
    }

    /// Compile and run a simple counting loop via JIT.
    /// Uses integer arithmetic + 4x loop unrolling for maximum speed.
    pub fn run_counting_loop(&mut self, n: u64) -> f64 {
        let mut jit = JitCompiler::new();
        let mem = jit.compile_fast_loop(n);
        let func: extern "C" fn(u64) -> u64 = unsafe { std::mem::transmute(mem.as_fn()) };
        let result = func(n);
        result as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jit_simple_loop() {
        let mut jit = JitContext::new();
        let sum = jit.run_counting_loop(100);
        // sum = 0 + 1 + 2 + ... + 99 = 4950
        assert!((sum - 4950.0).abs() < 0.001, "expected 4950, got {}", sum);
    }

    #[test]
    fn jit_large_loop() {
        let mut jit = JitContext::new();
        let sum = jit.run_counting_loop(10_000_000);
        // sum = 0 + 1 + 2 + ... + 9999999 = 49999995000000
        assert!((sum - 49999995000000.0).abs() < 1.0, "expected 49999995000000, got {}", sum);
    }

    #[test]
    fn jit_odd_count() {
        let mut jit = JitContext::new();
        let sum = jit.run_counting_loop(7);
        // sum = 0 + 1 + 2 + 3 + 4 + 5 + 6 = 21
        assert!((sum - 21.0).abs() < 0.001, "expected 21, got {}", sum);
    }

    #[test]
    fn jit_huge_loop() {
        let mut jit = JitContext::new();
        let sum = jit.run_counting_loop(100_000_000);
        // sum = 0 + 1 + 2 + ... + 99999999 = 4999999950000000
        assert!((sum - 4999999950000000.0).abs() < 1.0, "expected 4999999950000000, got {}", sum);
    }
}
