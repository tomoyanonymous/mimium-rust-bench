#![feature(test)]

extern crate test;

use osc_portaudiorust::replicate_faust::mydsp;
use osc_portaudiorust::replicate_libpd::LibpdDsp;
use osc_portaudiorust::replicate_mimium::MimiumProgram;
use std::hint::black_box;
use test::Bencher;

const SAMPLE_RATE: i32 = 48_000;
const FRAMES_PER_ITER: usize = 1024;

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
fn libpd_10_sine_oscillators(b: &mut Bencher) {
    let mut dsp = LibpdDsp::new(SAMPLE_RATE);
    let mut output = vec![0.0f32; FRAMES_PER_ITER];
    b.bytes = (FRAMES_PER_ITER * std::mem::size_of::<f32>()) as u64;

    b.iter(|| {
        dsp.process(&mut output);
        let checksum = checksum_words(output.iter().map(|s| s.to_bits() as u64));
        black_box(checksum)
    });
}