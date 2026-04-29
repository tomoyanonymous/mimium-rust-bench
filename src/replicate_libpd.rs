use std::ffi::CString;
use std::sync::OnceLock;

// libpd block size is always 64 samples
const LIBPD_BLOCK_SIZE: usize = 64;

static LIBPD_INIT: OnceLock<()> = OnceLock::new();

pub struct LibpdDsp {
    _patch: *mut std::ffi::c_void,
}

// libpd global state is initialized once per process; single-threaded bench use is safe
unsafe impl Send for LibpdDsp {}

impl LibpdDsp {
    pub fn new(sample_rate: i32) -> Self {
        LIBPD_INIT.get_or_init(|| unsafe {
            libpd_sys::libpd_init();
            libpd_sys::libpd_init_audio(0, 1, sample_rate);

            // send "pd dsp 1" to enable DSP processing
            libpd_sys::libpd_start_message(1);
            libpd_sys::libpd_add_float(1.0);
            let recv = CString::new("pd").unwrap();
            let msg = CString::new("dsp").unwrap();
            libpd_sys::libpd_finish_message(recv.as_ptr(), msg.as_ptr());
        });

        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let dir_cstr = CString::new(dir.to_str().unwrap()).unwrap();
        let file_cstr = CString::new("replicate.pd").unwrap();

        let patch = unsafe {
            libpd_sys::libpd_openfile(file_cstr.as_ptr(), dir_cstr.as_ptr())
        };
        assert!(!patch.is_null(), "failed to open replicate.pd");

        LibpdDsp { _patch: patch }
    }

    /// Process `frames` samples (must be a multiple of LIBPD_BLOCK_SIZE=64).
    /// `output` must have length == frames (1 channel, non-interleaved).
    pub fn process(&mut self, output: &mut [f32]) {
        debug_assert_eq!(output.len() % LIBPD_BLOCK_SIZE, 0);
        let ticks = (output.len() / LIBPD_BLOCK_SIZE) as i32;
        // 0 input channels: pass a valid non-null pointer to avoid UB
        let dummy_in = [0.0f32];
        unsafe {
            libpd_sys::libpd_process_float(ticks, dummy_in.as_ptr(), output.as_mut_ptr());
        }
    }
}
