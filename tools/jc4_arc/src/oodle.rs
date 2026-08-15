// Runtime binding to the game's Oodle DLL (oo2core_7_win64.dll) for codec-4 payloads.
// We load it dynamically (libloading) rather than link it — the DLL ships with the game and can't be
// redistributed. Signature is the standard public OodleLZ_Decompress ABI.
use libloading::{Library, Symbol};
use std::os::raw::c_void;

// intptr_t OodleLZ_Decompress(const void* comp, intptr_t compSize, void* raw, intptr_t rawLen,
//   int fuzzSafe, int checkCRC, int verbosity, void* decBufBase, intptr_t decBufSize,
//   void* fpCallback, void* callbackUserData, void* decoderMemory, intptr_t decoderMemorySize,
//   int threadPhase);  -> decompressed byte count (== rawLen on success), else 0/negative.
type OodleLZDecompress = unsafe extern "C" fn(
    *const c_void, isize, *mut c_void, isize,
    i32, i32, i32, *mut c_void, isize,
    *mut c_void, *mut c_void, *mut c_void, isize, i32,
) -> isize;

pub struct Oodle {
    _lib: Library,        // kept alive so `func` stays valid
    func: OodleLZDecompress,
}

impl Oodle {
    pub fn load(dll_path: &str) -> Result<Self, String> {
        unsafe {
            let lib = Library::new(dll_path).map_err(|e| format!("load {dll_path}: {e}"))?;
            let sym: Symbol<OodleLZDecompress> =
                lib.get(b"OodleLZ_Decompress\0").map_err(|e| format!("OodleLZ_Decompress: {e}"))?;
            let func = *sym;
            Ok(Oodle { _lib: lib, func })
        }
    }

    /// Decompress `comp` into exactly `out_len` bytes (the entry/block uncompressed size).
    pub fn decompress(&self, comp: &[u8], out_len: usize) -> Result<Vec<u8>, String> {
        let mut out = vec![0u8; out_len];
        // fuzzSafe=Yes(1), checkCRC=No(0), verbosity=None(0), threadPhase=Unthreaded(3), rest NULL/0.
        let n = unsafe {
            (self.func)(
                comp.as_ptr() as *const c_void, comp.len() as isize,
                out.as_mut_ptr() as *mut c_void, out_len as isize,
                1, 0, 0, std::ptr::null_mut(), 0,
                std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), 0, 3,
            )
        };
        if n as usize != out_len {
            return Err(format!("Oodle returned {n}, expected {out_len}"));
        }
        Ok(out)
    }
}
