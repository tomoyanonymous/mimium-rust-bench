use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use mimium_lang::{
    Config, ExecContext,
    function, numeric,
    interner::ToSymbol,
    plugin::{ExtClsInfo, InstantPlugin, Plugin},
    runtime::vm::Machine,
    types::Type,
};

const SRC: &str = include_str!("replicate.mmm");

fn make_runtime_plugin(sample_rate: Arc<AtomicU32>) -> InstantPlugin {
    let sr = sample_rate.clone();
    let getsamplerate = ExtClsInfo::new(
        "_mimium_getsamplerate".to_symbol(),
        function!(vec![], numeric!()),
        Rc::new(RefCell::new(move |machine: &mut Machine| {
            let rate = sr.load(Ordering::Relaxed) as f64;
            machine.set_stack(0, Machine::to_value(rate));
            1i64
        })),
    );

    // _mimium_getnow is used by schedulers; return 0 for DSP-only usage
    let counter = Arc::new(AtomicU64::new(0));
    let getnow = ExtClsInfo::new(
        "_mimium_getnow".to_symbol(),
        function!(vec![], numeric!()),
        Rc::new(RefCell::new(move |machine: &mut Machine| {
            let t = counter.fetch_add(1, Ordering::Relaxed) as f64;
            machine.set_stack(0, Machine::to_value(t));
            1i64
        })),
    );

    InstantPlugin {
        macros: vec![],
        extcls: vec![getsamplerate, getnow],
        commonfns: vec![],
    }
}

/// Mimium VM (bytecode interpreter) backend.
pub struct MimiumVmDsp {
    vm: Machine,
    dsp_i: usize,
    n_channels: usize,
}

impl MimiumVmDsp {
    pub fn new(sample_rate: u32) -> Self {
        let sr = Arc::new(AtomicU32::new(sample_rate));
        let plugin = make_runtime_plugin(sr);

        let mut ctx = ExecContext::new(
            std::iter::once(Box::new(plugin) as Box<dyn Plugin>),
            None,
            Config::default(),
        );
        ctx.prepare_machine(SRC).expect("mimium VM compile error");
        ctx.run_main();

        let vm = ctx.take_vm().expect("VM not initialized");
        let dsp_i = vm.prog.get_fun_index("dsp").expect("dsp function not found");
        let n_channels = vm.prog.iochannels.map_or(2, |io| io.output as usize);

        MimiumVmDsp { vm, dsp_i, n_channels }
    }

    /// Process samples into `output` (interleaved, n_channels per frame).
    pub fn process_buffer(&mut self, output: &mut [f64]) {
        for chunk in output.chunks_mut(self.n_channels) {
            self.vm.execute_idx(self.dsp_i);
            let raw = Machine::get_as_array::<f64>(self.vm.get_top_n(self.n_channels));
            chunk.copy_from_slice(raw);
        }
    }
}
