
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

```log
test faust_10_sine_oscillators  ... bench:      18,473.62 ns/iter (+/- 1,257.81) = 221 MB/s
test libpd_10_sine_oscillators  ... bench:      16,187.50 ns/iter (+/- 1,151.35) = 253 MB/s
test mimium_10_sine_oscillators ... bench:      50,399.36 ns/iter (+/- 1,026.78) = 325 MB/s
```

The MB/s figures are not directly comparable: Faust and libpd measure mono f32 throughput
(4 KB/iter), while mimium measures stereo f64 throughput (16 KB/iter).

## changed mimium processing resolution to f32

```log
test faust_10_sine_oscillators  ... bench:      18,557.24 ns/iter (+/- 1,588.95) = 220 MB/s
test libpd_10_sine_oscillators  ... bench:      16,634.76 ns/iter (+/- 213.76) = 246 MB/s
test mimium_10_sine_oscillators ... bench:      43,094.94 ns/iter (+/- 1,101.97) = 380 MB/s
```

## more memory load elimination

```log
test faust_10_sine_oscillators  ... bench:      18,484.36 ns/iter (+/- 1,984.58) = 221 MB/s
test libpd_10_sine_oscillators  ... bench:      16,641.27 ns/iter (+/- 646.23) = 246 MB/s
test mimium_10_sine_oscillators ... bench:      42,879.91 ns/iter (+/- 1,269.97) = 382 MB/s

```

## state operation condition check removed


```log
test faust_10_sine_oscillators  ... bench:      18,914.39 ns/iter (+/- 1,293.59) = 216 MB/s
test libpd_10_sine_oscillators  ... bench:      16,599.20 ns/iter (+/- 261.47) = 246 MB/s
test mimium_10_sine_oscillators ... bench:      33,201.89 ns/iter (+/- 1,018.78) = 493 MB/s
```
