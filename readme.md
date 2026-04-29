
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