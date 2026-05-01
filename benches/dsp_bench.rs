#![feature(test)]

extern crate test;

use mimium_transpiler_bench::replicate_faust::mydsp;
use mimium_transpiler_bench::replicate100_faust::mydsp as mydsp100;
use mimium_transpiler_bench::replicate_libpd::LibpdDsp;
use mimium_transpiler_bench::replicate_mimium::MimiumProgram;
use mimium_transpiler_bench::replicate100_mimium::MimiumProgram as MimiumProgram100;
use mimium_transpiler_bench::replicate_mimium_vm::MimiumVmDsp;
use mimium_transpiler_bench::replicate_mimium_wasm::MimiumWasmDsp;
use std::hint::black_box;
use test::Bencher;

const SAMPLE_RATE: i32 = 48_000;
const FRAMES_PER_ITER: usize = 1024;

const src10: &str = include_str!("../src/replicate.mmm");
const src100: &str = include_str!("../src/replicate100.mmm");

fn checksum_words(words: impl Iterator<Item = u64>) -> u64 {
    words.fold(0u64, |acc, word| acc ^ black_box(word.rotate_left(7)))
}

#[bench]
fn faust_10_sine_oscillators(b: &mut Bencher) {
    let mut dsp = mydsp::new();
    dsp.init(SAMPLE_RATE);

    let inputs: [&[f32]; 0] = [];
    let mut output = vec![0.0f32; FRAMES_PER_ITER];
    b.bytes = (FRAMES_PER_ITER * std::mem::size_of::<f32>()) as u64;

    b.iter(|| {
        let mut outputs = [&mut output[..]];
        dsp.compute(FRAMES_PER_ITER, &inputs, &mut outputs);
        let checksum = checksum_words(output.iter().map(|sample| sample.to_bits() as u64));
        black_box(checksum)
    });
}
#[bench]
fn faust_100_sine_oscillators(b: &mut Bencher) {
    let mut dsp = mydsp100::new();
    dsp.init(SAMPLE_RATE);

    let inputs: [&[f32]; 0] = [];
    let mut output = vec![0.0f32; FRAMES_PER_ITER];
    b.bytes = (FRAMES_PER_ITER * std::mem::size_of::<f32>()) as u64;

    b.iter(|| {
        let mut outputs = [&mut output[..]];
        dsp.compute(FRAMES_PER_ITER, &inputs, &mut outputs);
        let checksum = checksum_words(output.iter().map(|sample| sample.to_bits() as u64));
        black_box(checksum)
    });
}

#[bench]
fn mimium_10_sine_oscillators(b: &mut Bencher) {
    let mut program = MimiumProgram::new();
    let input: [f32; 0] = [];
    let mut output = vec![0.0f32; FRAMES_PER_ITER * 2];
    b.bytes = (FRAMES_PER_ITER * 2 * std::mem::size_of::<u64>()) as u64;

    b.iter(|| {
        program
            .call_dsp_buffer(&input, &mut output, FRAMES_PER_ITER)
            .unwrap();
        let checksum = checksum_words(output.chunks_exact(2).map(|frame| frame[0].to_bits() as u64));
        black_box(checksum)
    });
}
#[bench]
fn mimium_100_sine_oscillators(b: &mut Bencher) {
    let mut program = MimiumProgram100::new();
    let input: [f32; 0] = [];
    let mut output = vec![0.0f32; FRAMES_PER_ITER * 2];
    b.bytes = (FRAMES_PER_ITER * 2 * std::mem::size_of::<u64>()) as u64;

    b.iter(|| {
        program
            .call_dsp_buffer(&input, &mut output, FRAMES_PER_ITER)
            .unwrap();
        let checksum = checksum_words(output.chunks_exact(2).map(|frame| frame[0].to_bits() as u64));
        black_box(checksum)
    });
}
#[bench]
fn mimium_vm_10_sine_oscillators(b: &mut Bencher) {
    let mut dsp = MimiumVmDsp::new(SAMPLE_RATE as u32, src10);
    let mut output = vec![0.0f64; FRAMES_PER_ITER * 2];
    b.bytes = (FRAMES_PER_ITER * 2 * std::mem::size_of::<f64>()) as u64;

    b.iter(|| {
        dsp.process_buffer(&mut output);
        let checksum = checksum_words(output.chunks_exact(2).map(|frame| frame[0].to_bits()));
        black_box(checksum)
    });
}

#[bench]
fn mimium_wasm_10_sine_oscillators(b: &mut Bencher) {
    let mut dsp = MimiumWasmDsp::new(SAMPLE_RATE as f64, src10);
    let mut output = vec![0.0f64; FRAMES_PER_ITER * 2];
    b.bytes = (FRAMES_PER_ITER * 2 * std::mem::size_of::<f64>()) as u64;

    b.iter(|| {
        dsp.process_buffer(&mut output);
        let checksum = checksum_words(output.chunks_exact(2).map(|frame| frame[0].to_bits()));
        black_box(checksum)
    });
}
#[bench]
fn mimium_vm_100_sine_oscillators(b: &mut Bencher) {
    let mut dsp = MimiumVmDsp::new(SAMPLE_RATE as u32, src100);
    let mut output = vec![0.0f64; FRAMES_PER_ITER * 2];
    b.bytes = (FRAMES_PER_ITER * 2 * std::mem::size_of::<f64>()) as u64;

    b.iter(|| {
        dsp.process_buffer(&mut output);
        let checksum = checksum_words(output.chunks_exact(2).map(|frame| frame[0].to_bits()));
        black_box(checksum)
    });
}

#[bench]
fn mimium_wasm_100_sine_oscillators(b: &mut Bencher) {
    let mut dsp = MimiumWasmDsp::new(SAMPLE_RATE as f64, src100);
    let mut output = vec![0.0f64; FRAMES_PER_ITER * 2];
    b.bytes = (FRAMES_PER_ITER * 2 * std::mem::size_of::<f64>()) as u64;

    b.iter(|| {
        dsp.process_buffer(&mut output);
        let checksum = checksum_words(output.chunks_exact(2).map(|frame| frame[0].to_bits()));
        black_box(checksum)
    });
}

#[bench]
fn libpd_10_sine_oscillators(b: &mut Bencher) {
    let mut dsp = LibpdDsp::new(SAMPLE_RATE,"replicate.pd");
    let mut output = vec![0.0f32; FRAMES_PER_ITER];
    b.bytes = (FRAMES_PER_ITER * std::mem::size_of::<f32>()) as u64;

    b.iter(|| {
        dsp.process(&mut output);
        let checksum = checksum_words(output.iter().map(|s| s.to_bits() as u64));
        black_box(checksum)
    });
}
#[bench]
fn libpd_100_sine_oscillators(b: &mut Bencher) {
    let mut dsp = LibpdDsp::new(SAMPLE_RATE,"replicate_clone100.pd");
    let mut output = vec![0.0f32; FRAMES_PER_ITER];
    b.bytes = (FRAMES_PER_ITER * std::mem::size_of::<f32>()) as u64;

    b.iter(|| {
        dsp.process(&mut output);
        let checksum = checksum_words(output.iter().map(|s| s.to_bits() as u64));
        black_box(checksum)
    });
}