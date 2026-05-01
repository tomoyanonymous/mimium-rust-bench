use mimium_lang::{
    Config, ExecContext,
    runtime::{DspRuntime, Time},
    runtime::wasm::engine::{WasmDspRuntime, WasmEngine},
};

const SRC: &str = include_str!("replicate.mmm");

/// Mimium Wasm (wasmtime JIT) backend.
pub struct MimiumWasmDsp {
    runtime: WasmDspRuntime,
    n_channels: usize,
}

impl MimiumWasmDsp {
    pub fn new(sample_rate: f64) -> Self {
        let mut ctx = ExecContext::new(std::iter::empty(), None, Config::default());
        ctx.prepare_compiler();
        let wasm_output = ctx
            .get_compiler()
            .unwrap()
            .emit_wasm(SRC)
            .expect("mimium Wasm compile error");

        let n_channels = wasm_output.io_channels.map_or(2, |io| io.output as usize);

        let mut engine = WasmEngine::new(&wasm_output.ext_fns, None)
            .expect("failed to create WasmEngine");
        engine
            .load_module(&wasm_output.bytes)
            .expect("failed to load Wasm module");

        let mut runtime = WasmDspRuntime::new(
            engine,
            wasm_output.io_channels,
            wasm_output.dsp_state_skeleton,
        );
        runtime.set_sample_rate(sample_rate);
        runtime.run_main().expect("failed to run mimium main (Wasm)");

        MimiumWasmDsp { runtime, n_channels }
    }

    /// Process `frames` samples into `output` (interleaved, n_channels per frame).
    pub fn process_buffer(&mut self, output: &mut [f64]) {
        for chunk in output.chunks_mut(self.n_channels) {
            self.runtime.run_dsp(Time(0));
            chunk.copy_from_slice(self.runtime.get_output(self.n_channels));
        }
    }
}
