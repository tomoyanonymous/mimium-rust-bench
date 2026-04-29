
# Benchmark of mimium rust codegen


to use instuments:

```
cargo install cargo-instruments
cargo +nightly instruments --bench dsp_bench -t time --profile=dev
```

```
cargo +nightly flamegraph --dev --bench dsp_bench
```


## Initial result

```log
test faust_10_sine_oscillators  ... bench:      18,507.43 ns/iter (+/- 1,049.02) = 221 MB/s
test mimium_10_sine_oscillators ... bench:     325,204.56 ns/iter (+/- 16,998.27) = 50 MB/s
```

## Frame-wide execution in mimium code


```log
test faust_10_sine_oscillators  ... bench:      18,617.05 ns/iter (+/- 747.16) = 220 MB/s
test mimium_10_sine_oscillators ... bench:     303,513.67 ns/iter (+/- 42,895.46) = 53 MB/s
```

## Forcing inlining

```log
test faust_10_sine_oscillators  ... bench:      18,517.71 ns/iter (+/- 860.10) = 221 MB/s
test mimium_10_sine_oscillators ... bench:     271,811.46 ns/iter (+/- 25,345.55) = 60 MB/s
```

## typed register

```log
test faust_10_sine_oscillators  ... bench:      18,421.41 ns/iter (+/- 1,021.47) = 222 MB/s
test mimium_10_sine_oscillators ... bench:     267,846.74 ns/iter (+/- 30,923.32) = 61 MB/s
```

## supress heap memory slot

```log
test faust_10_sine_oscillators  ... bench:      18,775.07 ns/iter (+/- 1,060.44) = 218 MB/s
test mimium_10_sine_oscillators ... bench:     130,797.50 ns/iter (+/- 21,112.80) = 125 MB/s
```

## direct aggreagte return

```log
test faust_10_sine_oscillators  ... bench:      18,753.44 ns/iter (+/- 13,774.10) = 218 MB/s
test mimium_10_sine_oscillators ... bench:      56,683.68 ns/iter (+/- 13,491.20) = 289 MB/s
```

## fixed unused casts

```log
test faust_10_sine_oscillators  ... bench:      18,530.96 ns/iter (+/- 936.06) = 221 MB/s
test mimium_10_sine_oscillators ... bench:      50,977.52 ns/iter (+/- 1,494.50) = 321 MB/s
```

## Pure Data comparison via libpd-sys

Added `libpd_10_sine_oscillators` benchmark using [libpd-sys](https://crates.io/crates/libpd-sys).
The Pure Data patch (`src/replicate.pd`) uses 10 `osc~` objects at 50–500 Hz (in 50 Hz steps)
with amplitude scaling 1/n, summed to a single output channel.

> **Note on algorithm**: PD's `osc~` is a cosine wavetable oscillator (512-entry table),
> whereas Faust and mimium compute `sin()` directly. Wavetable lookup is significantly cheaper
> than a transcendental function call, which largely explains the speed difference below.

```log
test faust_10_sine_oscillators  ... bench:      18,541.49 ns/iter (+/- 1,171.40) = 220 MB/s
test libpd_10_sine_oscillators  ... bench:      12,684.79 ns/iter (+/-   282.24) = 322 MB/s
test mimium_10_sine_oscillators ... bench:      50,358.47 ns/iter (+/- 2,419.06) = 325 MB/s
```

The MB/s figures are not directly comparable: Faust and libpd measure mono f32 throughput
(4 KB/iter), while mimium measures stereo f64 throughput (16 KB/iter).