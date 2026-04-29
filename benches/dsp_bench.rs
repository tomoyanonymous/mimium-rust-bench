#![feature(test)]

extern crate test;

use osc_portaudiorust::replicate_faust::mydsp;
use osc_portaudiorust::replicate_mimium::MimiumProgram;
use std::hint::black_box;
use test::Bencher;

const SAMPLE_RATE: i32 = 48_000;
const FRAMES_PER_ITER: usize = 1024;

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
        let checksum = output
            .iter()
            .fold(0u32, |acc, sample| acc ^ black_box(sample.to_bits()));
        black_box(checksum)
    });
}

#[bench]
fn mimium_10_sine_oscillators(b: &mut Bencher) {
    let mut program = MimiumProgram::new();
    b.bytes = (FRAMES_PER_ITER * 2 * std::mem::size_of::<u64>()) as u64;

    b.iter(|| {
        let mut checksum = 0u64;
        for _ in 0..FRAMES_PER_ITER {
            let (left, right) = program.dsp_step_raw();
            checksum ^= black_box(left.rotate_left(7) ^ right);
        }
        black_box(checksum)
    });
}